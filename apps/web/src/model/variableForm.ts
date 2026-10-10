import type { ProjectVariable } from '../contracts/generated/ProjectVariable';
import type { VariableKind } from '../contracts/generated/VariableKind';
import type { VariableValue } from '../contracts/generated/VariableValue';
import { number } from './definitionForm';
import { isIsoDate, VARIABLES_MAX, VARIABLE_TEXT_MAX, variableKey, variableNameProblem } from './projectVariables';

/**
 * Proje ayarları › Değişkenler's form (docs/adr/0214 §2.3, §4): the project's variables as rows of texts (name, label,
 * kind, value), the rows read back into variables with what is said of a row that does not hold, the row a new variable
 * starts as and a row whose kind changes. A name is a letter or `_`, then letters, digits and `_`, compared with Turkish
 * letters and case aside; a value is typed as its kind reads it (a number as the Hesap windows read one, a date as
 * YYYY-AA-GG or GG.AA.YYYY, true/false from the switch). The desktop's twin is `kentos_project::variable_form`; both pass
 * fixtures/project/v1/variable-form.json (scripts/fixtures/variable_form_cases.py).
 */

/** What is said of a row that does not hold. */
export const VARIABLE_TEXTS = {
  nameEmpty: 'Bir ad yazın: harf ya da _ ile başlar, harf, rakam ve _ içerir.',
  number: 'Sayı yazın (1.5 ya da 1,5); değeri yoksa boş bırakın.',
  date: 'Tarihi YYYY-AA-GG ya da GG.AA.YYYY yazın; değeri yoksa boş bırakın.',
  bool: 'Doğru ya da yanlış seçin.',
  tooMany: 'En çok 200 değişken olabilir.',
  textLong: 'Değer 4000 karakterden uzun.',
} as const;

/** The switch's two values, as a row keeps them. */
export const TRUE_TEXT = 'doğru';
export const FALSE_TEXT = 'yanlış';

/** One variable as the form types it. */
export interface VariableRow {
  name: string;
  label: string;
  kind: VariableKind;
  value: string;
}

/** What does not hold in a row: its name, its value. */
export interface RowProblems {
  readonly name: string | null;
  readonly value: string | null;
}

/** The rows read back: the variables that hold, each row's problems, too many rows. */
export interface VariablesRead {
  readonly variables: ProjectVariable[];
  readonly problems: RowProblems[];
  readonly list: string | null;
}

/** Whether a row has something to put right: Kaydet waits. */
export const variablesBlocked = (r: VariablesRead): boolean => r.list !== null || r.problems.some((p) => p.name !== null || p.value !== null);

/** A number as the form writes it: the shortest text that reads back as it, without an exponent; -0 is "0". */
export function numberText(x: number): string {
  if (x === 0) return '0';
  const s = String(x);
  const m = /^(-?)(\d)(?:\.(\d+))?e([+-]\d+)$/.exec(s);
  if (!m) return s;
  const [, sign, lead, rest = '', exp] = m;
  const digits = lead + rest;
  const point = 1 + Number(exp);
  if (point <= 0) return `${sign}0.${'0'.repeat(-point)}${digits}`;
  return point >= digits.length ? `${sign}${digits}${'0'.repeat(point - digits.length)}` : `${sign}${digits.slice(0, point)}.${digits.slice(point)}`;
}

/** A variable's value as the form writes it for its kind. */
export function valueText(value: VariableValue | undefined): string {
  if (value === null || value === undefined) return '';
  if (typeof value === 'boolean') return value ? TRUE_TEXT : FALSE_TEXT;
  if (typeof value === 'number') return numberText(value);
  return value;
}

/** The form's rows of a project's variables. */
export const variableRows = (list: readonly ProjectVariable[]): VariableRow[] =>
  list.map((v) => ({ name: v.name, label: v.label ?? '', kind: v.kind ?? 'text', value: valueText(v.value) }));

/** `G.A.YYYY` or `GG.AA.YYYY` as `YYYY-AA-GG`; null for anything else. */
function dotted(t: string): string | null {
  const m = /^(\d{1,2})\.(\d{1,2})\.(\d{4})$/.exec(t);
  return m ? `${m[3]}-${m[2].padStart(2, '0')}-${m[1].padStart(2, '0')}` : null;
}

/** A value typed for a kind, or what is said of it; a blank text is no value. */
export function readValue(kind: VariableKind, typed: string): { value: VariableValue } | { problem: string } {
  const t = typed.trim();
  if (!t) return { value: null };
  switch (kind) {
    case 'text':
      return [...t].length > VARIABLE_TEXT_MAX ? { problem: VARIABLE_TEXTS.textLong } : { value: t };
    case 'number': {
      const x = number(t);
      return x !== null && Number.isFinite(x) ? { value: x } : { problem: VARIABLE_TEXTS.number };
    }
    case 'date': {
      const iso = isIsoDate(t) ? t : dotted(t);
      return iso !== null && isIsoDate(iso) ? { value: iso } : { problem: VARIABLE_TEXTS.date };
    }
    default:
      return t === TRUE_TEXT ? { value: true } : t === FALSE_TEXT ? { value: false } : { problem: VARIABLE_TEXTS.bool };
  }
}

/** The rows read back: the variables that hold and every row's problems. */
export function readVariables(rows: readonly VariableRow[]): VariablesRead {
  const variables: ProjectVariable[] = [];
  const problems: RowProblems[] = [];
  const seen = new Set<string>();
  for (const r of rows) {
    const trimmed = r.name.trim();
    const name = trimmed.startsWith('@') ? trimmed.slice(1) : trimmed;
    const named = name === '' ? VARIABLE_TEXTS.nameEmpty : variableNameProblem(name);
    let nameProblem = named === null ? null : name === '' ? named : `${named}.`;
    if (nameProblem === null) {
      const key = variableKey(name);
      if (seen.has(key)) nameProblem = `@${name} adı yukarıda var; başka ad seçin.`;
      else seen.add(key);
    }
    const read = readValue(r.kind, r.value);
    if (nameProblem === null && 'value' in read) {
      const label = r.label.trim();
      // No value is none written, as the project keeps it.
      variables.push({ name, ...(label ? { label } : {}), kind: r.kind, ...(read.value !== null ? { value: read.value } : {}) });
    }
    problems.push({ name: nameProblem, value: 'problem' in read ? read.problem : null });
  }
  return { variables, problems, list: rows.length > VARIABLES_MAX ? VARIABLE_TEXTS.tooMany : null };
}

/** The row a new variable starts as: the first free name of `degisken1`, `degisken2` …, no label, text, no value. */
export function newVariableRow(rows: readonly VariableRow[]): VariableRow {
  const taken = new Set(rows.map((r) => variableKey(r.name.trim().replace(/^@+/, ''))));
  let n = 1;
  while (taken.has(variableKey(`degisken${n}`))) n += 1;
  return { name: `degisken${n}`, label: '', kind: 'text', value: '' };
}

/**
 * A row whose kind changes: to true/false its value is "doğru" when it was, else "yanlış"; from true/false it is
 * emptied; between the others the text stays (reading it says whether it holds).
 */
export function withKind(row: VariableRow, kind: VariableKind): VariableRow {
  const value = kind === 'bool' ? (row.value.trim() === TRUE_TEXT ? TRUE_TEXT : FALSE_TEXT) : row.kind === 'bool' ? '' : row.value;
  return { ...row, kind, value };
}
