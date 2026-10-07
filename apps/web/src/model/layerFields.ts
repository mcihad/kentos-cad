import type { FieldChoice } from '../contracts/generated/FieldChoice';
import type { LayerField } from '../contracts/generated/LayerField';
import type { LayerFieldKind } from '../contracts/generated/LayerFieldKind';
import { naturalOrder } from './ops/pointEditor';

/**
 * A layer's fields (docs/adr/0199 §1, §2): the schema a layer may give its objects' attributes. Values stay text; a
 * field says which text it takes and in what one form (its canonical text), which the product commands write, the
 * attribute table sorts by and the forms edit. The contract's rules (`kentos_contracts::fields`) written again for the
 * web's commands; both pass the independent reference's cases (scripts/fixtures/layer_field_cases.py,
 * fixtures/layer-fields/v1/cases.json) word for word.
 */

export type { FieldChoice, LayerField, LayerFieldKind };

/** The largest whole number a field takes, in magnitude: 2⁵³ − 1. */
export const MAX_INTEGER = '9007199254740991';
/** The most digits a decimal value has, whole and fraction together. */
export const MAX_DIGITS = 30;
/** The most characters of a field's name and of its alias. */
export const MAX_NAME = 64;
/** A text field's longest length. */
export const MAX_LENGTH = 10000;
/** A decimal field's most fraction digits. */
export const MAX_SCALE = 15;

/** Every kind, in the order the Alanlar window lists them, with its name in the interface. */
export const FIELD_KINDS: readonly { kind: LayerFieldKind; label: string }[] = [
  { kind: 'text', label: 'Metin' },
  { kind: 'integer', label: 'Tam sayı' },
  { kind: 'decimal', label: 'Ondalık sayı' },
  { kind: 'date', label: 'Tarih' },
  { kind: 'boolean', label: 'Evet/hayır' },
];

/** A kind's name in the interface. */
export const kindLabel = (kind: LayerFieldKind): string => FIELD_KINDS.find((k) => k.kind === kind)?.label ?? kind;

/** Whether a kind's values are numbers (a range and a value list take them). */
export const isNumberKind = (kind: LayerFieldKind): boolean => kind === 'integer' || kind === 'decimal';

/** The name a message, the table and the form give a field: its alias when it has one. */
export const fieldLabel = (f: LayerField): string => (f.alias ? f.alias : f.name);

/** Why a value does not go into a field: a short code and the sentence that says it. */
export interface FieldRefusal {
  code: 'type' | 'magnitude' | 'scale' | 'length' | 'required' | 'choice' | 'range';
  message: string;
}

export type FieldCheck = { value: string } | { error: FieldRefusal['code']; message: string };

/** A letter folded the Turkish way (docs/adr/0178 §2): I is ı's, İ is i's, any other its lowercase when that is one character. */
const foldChar = (c: string): string => {
  if (c === 'I') return 'ı';
  if (c === 'İ') return 'i';
  const low = c.toLowerCase();
  return [...low].length === 1 ? low : c;
};

/** `text` folded the Turkish way, letter by letter. */
export const fold = (text: string): string => [...text].map(foldChar).join('');

const chars = (text: string): number => [...text].length;

// ── Kinds ──────────────────────────────────────────────────────────────

/** A whole number's canonical text, `'big'` past MAX_INTEGER, null when `s` is no whole number. */
function integer(s: string): string | 'big' | null {
  const m = /^([+-]?)([0-9]+)$/.exec(s);
  if (!m) return null;
  const rest = m[2].replace(/^0+/, '');
  if (!rest) return '0';
  if (rest.length > 16 || (rest.length === 16 && rest > MAX_INTEGER)) return 'big';
  return m[1] === '-' ? `-${rest}` : rest;
}

/** A decimal number as read: its canonical text, how many digits it was written with and how many after the separator. */
function decimal(s: string): { text: string; digits: number; fraction: number } | null {
  const m = /^([+-]?)([0-9]*)(?:[.,]([0-9]*))?$/.exec(s);
  if (!m) return null;
  const whole = m[2];
  const fraction = m[3] ?? '';
  if (!whole && !fraction) return null;
  const digits = whole.length + fraction.length;
  const w = whole.replace(/^0+/, '') || '0';
  const zero = w === '0' && /^0*$/.test(fraction);
  const text = (m[1] === '-' && !zero ? '-' : '') + w + (fraction ? `.${fraction}` : '');
  return { text, digits, fraction: fraction.length };
}

const leap = (y: number): boolean => y % 4 === 0 && (y % 100 !== 0 || y % 400 === 0);
const daysIn = (y: number, m: number): number => (m === 2 ? (leap(y) ? 29 : 28) : [4, 6, 9, 11].includes(m) ? 30 : 31);

/** A date's canonical text (YYYY-AA-GG) from YYYY-AA-GG or GG.AA.YYYY (day and month one or two digits). */
function date(s: string): string | null {
  let y: number, mo: number, d: number;
  const iso = /^([0-9]{4})-([0-9]{2})-([0-9]{2})$/.exec(s);
  if (iso) [y, mo, d] = [Number(iso[1]), Number(iso[2]), Number(iso[3])];
  else {
    const tr = /^([0-9]{1,2})\.([0-9]{1,2})\.([0-9]{4})$/.exec(s);
    if (!tr) return null;
    [d, mo, y] = [Number(tr[1]), Number(tr[2]), Number(tr[3])];
  }
  if (y < 1 || y > 9999 || mo < 1 || mo > 12 || d < 1 || d > daysIn(y, mo)) return null;
  return `${String(y).padStart(4, '0')}-${String(mo).padStart(2, '0')}-${String(d).padStart(2, '0')}`;
}

function boolean(s: string): 'true' | 'false' | null {
  const f = fold(s);
  if (f === 'evet' || f === 'true' || f === '1') return 'true';
  if (f === 'hayır' || f === 'false' || f === '0') return 'false';
  return null;
}

/** Two decimal numbers' order by their exact values (−1, 0, 1); null when either is not one. */
export function compareDecimals(a: string, b: string): number | null {
  const parts = (s: string): [number, string, string] | null => {
    const d = decimal(s);
    if (!d) return null;
    const minus = d.text.startsWith('-');
    const [whole, frac = ''] = (minus ? d.text.slice(1) : d.text).split('.');
    const fraction = frac.replace(/0+$/, '');
    const sign = whole === '0' && !fraction ? 0 : minus ? -1 : 1;
    return [sign, whole, fraction];
  };
  const pa = parts(a);
  const pb = parts(b);
  if (!pa || !pb) return null;
  const [sa, wa, fa] = pa;
  const [sb, wb, fb] = pb;
  if (sa !== sb) return sa < sb ? -1 : 1;
  const cmp = (x: string, y: string): number => (x < y ? -1 : x > y ? 1 : 0);
  const magnitude = wa.length !== wb.length ? (wa.length < wb.length ? -1 : 1) : cmp(wa, wb) || cmp(fa, fb);
  return sa < 0 ? -magnitude : magnitude;
}

/** The kind's canonical text of trimmed `s` (`text` as given, for the messages). */
function kindValue(f: LayerField, s: string, text: string): FieldCheck {
  const L = fieldLabel(f);
  switch (f.kind) {
    case 'integer': {
      const v = integer(s);
      if (v === null) return { error: 'type', message: `“${L}” alanı tam sayı ister; “${text}” verildi. Rakamlarla, ondalıksız yazın.` };
      if (v === 'big') return { error: 'magnitude', message: `“${L}” alanının sayısı çok büyük; “${text}” verildi. Mutlak değeri en çok ${MAX_INTEGER} olabilir.` };
      return { value: v };
    }
    case 'decimal': {
      const d = decimal(s);
      if (!d) return { error: 'type', message: `“${L}” alanı ondalık sayı ister; “${text}” verildi. Rakamlarla, ondalığı nokta ya da virgülle yazın.` };
      if (d.digits > MAX_DIGITS) return { error: 'magnitude', message: `“${L}” alanının sayısı çok uzun; “${text}” verildi. En çok ${MAX_DIGITS} rakam olabilir.` };
      if (f.scale !== undefined && d.fraction > f.scale)
        return { error: 'scale', message: `“${L}” alanı en çok ${f.scale} ondalık basamak alır; “${text}” verildi. Değer yuvarlanmaz; basamakları azaltarak yazın.` };
      return { value: d.text };
    }
    case 'date': {
      const v = date(s);
      return v === null ? { error: 'type', message: `“${L}” alanı tarih ister; “${text}” verildi. GG.AA.YYYY ya da YYYY-AA-GG yazın.` } : { value: v };
    }
    case 'boolean': {
      const v = boolean(s);
      return v === null ? { error: 'type', message: `“${L}” alanı evet ya da hayır ister; “${text}” verildi.` } : { value: v };
    }
    default:
      return { value: text };
  }
}

const choicesOf = (f: LayerField): FieldChoice[] | null => (f.values && f.values.length ? f.values : null);

/**
 * A value written to `field` (docs/adr/0199 §1): its canonical text, or why it does not go in. Empty (blank) is `""`
 * unless the field is required; a value list's label (folded, trimmed) gives its code; a range is exact, its ends included.
 */
export function checkValue(f: LayerField, text: string): FieldCheck {
  const L = fieldLabel(f);
  const s = text.trim();
  if (!s) return f.required ? { error: 'required', message: `“${L}” alanı zorunlu; boş bırakılamaz.` } : { value: '' };
  if (f.kind === 'text' && f.length !== undefined) {
    const n = chars(text);
    if (n > f.length) return { error: 'length', message: `“${L}” alanı en çok ${f.length} karakter alır; ${n} karakter verildi.` };
  }
  const choices = choicesOf(f);
  if (choices) {
    const wanted = fold(s);
    const c = choices.find((x) => fold(x.label.trim()) === wanted);
    if (c) return { value: c.code };
  }
  const r = kindValue(f, s, text);
  if ('error' in r) return r;
  // A text field's code is matched as trimmed; without a list the text stays as given.
  const value = f.kind === 'text' && choices ? s : r.value;
  if (choices && !choices.some((c) => c.code === value))
    return { error: 'choice', message: `“${L}” alanı listedeki değerlerden birini ister; “${text}” listede yok.` };
  if (isNumberKind(f.kind)) {
    const below = f.min !== undefined && compareDecimals(value, f.min) === -1;
    const above = f.max !== undefined && compareDecimals(value, f.max) === 1;
    if (below || above) {
      const message =
        f.min !== undefined && f.max !== undefined
          ? `“${L}” alanı ${f.min} ile ${f.max} arasında olmalı; ${value} verildi.`
          : f.min !== undefined
            ? `“${L}” alanı en az ${f.min} olmalı; ${value} verildi.`
            : `“${L}” alanı en çok ${f.max ?? ''} olmalı; ${value} verildi.`;
      return { error: 'range', message };
    }
  }
  return { value };
}

/** Whether `v` is already the canonical text of the field's kind (its scale counted): a range's end, a number's code. */
function canonicalOf(f: LayerField, v: string): boolean {
  const s = v.trim();
  if (!s) return false;
  const r = kindValue(f, s, v);
  return 'value' in r && r.value === v;
}

// ── A field list's problems ────────────────────────────────────────────

/** What is wrong with a field, when anything is (docs/adr/0199 §1), in the words the contract says it. */
export function fieldProblem(f: LayerField): string | null {
  const name = f.name;
  if (!name.trim()) return 'Alanın adı boş olamaz.';
  if (name.trim() !== name) return `“${name}” alanının adının başında ya da sonunda boşluk var.`;
  if (chars(name) > MAX_NAME) return `“${name}” alanının adı en çok ${MAX_NAME} karakter olabilir.`;
  if ([...name].some((c) => c.codePointAt(0)! < 32 || (c.codePointAt(0)! >= 127 && c.codePointAt(0)! < 160))) return `“${name}” alanının adında denetim karakteri var.`;
  if (f.alias !== undefined) {
    if (!f.alias.trim()) return `“${name}” alanının takma adı boş olamaz.`;
    if (chars(f.alias) > MAX_NAME) return `“${name}” alanının takma adı en çok ${MAX_NAME} karakter olabilir.`;
  }
  const kind = f.kind;
  if (f.length !== undefined) {
    if (kind !== 'text') return `“${name}” alanında uzunluk yalnız metin alanında olur.`;
    if (f.length < 1 || f.length > MAX_LENGTH) return `“${name}” alanının uzunluğu 1 ile ${MAX_LENGTH} arasında olmalı.`;
  }
  if (f.scale !== undefined) {
    if (kind !== 'decimal') return `“${name}” alanında ondalık basamak yalnız ondalık sayı alanında olur.`;
    if (f.scale < 0 || f.scale > MAX_SCALE) return `“${name}” alanının ondalık basamağı 0 ile ${MAX_SCALE} arasında olmalı.`;
  }
  if (f.min !== undefined || f.max !== undefined) {
    if (!isNumberKind(kind)) return `“${name}” alanında aralık yalnız sayı alanlarında olur.`;
    for (const [end, word] of [
      [f.min, 'en azı'],
      [f.max, 'en çoğu'],
    ] as const)
      if (end !== undefined && !canonicalOf(f, end)) return `“${name}” alanının ${word} “${end}” alanın türüne uymuyor.`;
    if (f.min !== undefined && f.max !== undefined && compareDecimals(f.min, f.max) === 1) return `“${name}” alanının en azı en çoğundan büyük olamaz.`;
  }
  if (f.values !== undefined) {
    if (kind !== 'text' && !isNumberKind(kind)) return `“${name}” alanında değer listesi yalnız metin ve sayı alanlarında olur.`;
    if (!f.values.length) return `“${name}” alanının değer listesi boş; listeyi kaldırın ya da değer ekleyin.`;
    for (let i = 0; i < f.values.length; i++) {
      const { code, label } = f.values[i];
      if (!code) return `“${name}” alanının değer listesinde boş kod var.`;
      if (kind !== 'text' && !canonicalOf(f, code)) return `“${name}” alanının değer listesindeki “${code}” kodu alanın türüne uymuyor.`;
      if (!label.trim()) return `“${name}” alanının değer listesinde “${code}” kodunun etiketi boş.`;
      const before = f.values.slice(0, i);
      if (before.some((w) => w.code === code)) return `“${name}” alanının değer listesinde “${code}” kodu iki kez var.`;
      const key = fold(label.trim());
      if (before.some((w) => fold(w.label.trim()) === key)) return `“${name}” alanının değer listesinde “${label}” etiketi iki kez var.`;
    }
  }
  if (f.default !== undefined) {
    const d = f.default;
    const r = checkValue({ ...f, required: false }, d);
    if (!d.trim() || 'error' in r || r.value !== d) return `“${name}” alanının varsayılanı “${d}” alanın kurallarına uymuyor.`;
  }
  return null;
}

/** The first problem of a layer's fields: one field's, or a name used twice (folded the Turkish way). */
export function fieldsProblem(fields: readonly LayerField[]): string | null {
  const seen = new Set<string>();
  for (const f of fields) {
    const p = fieldProblem(f);
    if (p) return p;
    const key = fold(f.name);
    if (seen.has(key)) return `“${f.name}” adlı iki alan var; alan adları bir kez kullanılır.`;
    seen.add(key);
  }
  return null;
}

/** The problem of a layer's stored fields: `fieldsProblem`'s, or an empty list (a layer without fields does not write one). */
export function layerFieldsProblem(fields: readonly LayerField[]): string | null {
  if (!fields.length) return 'katmanın alan listesi boş olamaz; alanı yoksa yazılmaz';
  return fieldsProblem(fields);
}

// ── Inferred fields and display ────────────────────────────────────────

/**
 * The field Verilerden al gives a key whose values are `values` (docs/adr/0199 §3): the first kind every value that is
 * not blank takes, whole numbers, decimals (with as many fraction digits as the longest has, at most MAX_SCALE), dates
 * and yes or no words in that order, else text.
 */
export function inferField(name: string, values: Iterable<string>): LayerField {
  const vs = [...values].map((v) => v.trim()).filter((v) => v);
  if (!vs.length) return { name, kind: 'text' };
  if (vs.every((v) => { const n = integer(v); return n !== null && n !== 'big'; })) return { name, kind: 'integer' };
  const decimals = vs.map((v) => decimal(v));
  if (decimals.every((d) => d !== null && d.digits <= MAX_DIGITS)) {
    const scale = Math.max(...decimals.map((d) => d!.fraction));
    return scale > MAX_SCALE ? { name, kind: 'text' } : { name, kind: 'decimal', scale };
  }
  if (vs.every((v) => date(v) !== null)) return { name, kind: 'date' };
  if (vs.every((v) => ['evet', 'hayır', 'true', 'false'].includes(fold(v)))) return { name, kind: 'boolean' };
  return { name, kind: 'text' };
}

/** The fields Verilerden al gives objects' attributes: one per key, in the natural order (docs/adr/0153 §6). */
export function inferFields(rows: Iterable<Readonly<Record<string, string>>>): LayerField[] {
  const keys = new Map<string, string[]>();
  for (const row of rows)
    for (const [k, v] of Object.entries(row)) {
      const list = keys.get(k);
      if (list) list.push(v);
      else keys.set(k, [v]);
    }
  const names = [...keys.keys()];
  return naturalOrder(names).map((i) => inferField(names[i], keys.get(names[i])!));
}

/** How a value of a field is shown: a code its label, a yes or no Evet or Hayır, a date GG.AA.YYYY, anything else as it is. */
export function displayValue(f: LayerField, value: string): string {
  const c = f.values?.find((x) => x.code === value);
  if (c) return c.label;
  if (f.kind === 'boolean') return value === 'true' ? 'Evet' : value === 'false' ? 'Hayır' : value;
  if (f.kind === 'date' && /^[0-9]{4}-[0-9]{2}-[0-9]{2}$/.test(value)) return `${value.slice(8, 10)}.${value.slice(5, 7)}.${value.slice(0, 4)}`;
  return value;
}
