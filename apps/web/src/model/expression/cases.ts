import type { Entity } from '../entities';
import type { Gen } from '../../wasm/calls/harness';
import { MEASURE_STRIDE } from './expressionLib';

/**
 * Test support for the expression language (docs/adr/0008 “İfade dili”):
 * random objects and random sources, valid and broken, for the comparison
 * with the TypeScript it replaced and for the frozen cases
 * (fixtures/expression/v1/cases.json, scripts/fixtures/record-expression.test.ts).
 * Characters outside the Basic Multilingual Plane are left out: JavaScript
 * keeps half of a cut emoji as a lone surrogate, Rust text cannot.
 */

/** Attribute names: bare words and names that need brackets. */
const WORDS = ['Parsel', 'Ada', 'Nitelik', 'Kat', 'Boş', 'Yok', 'Ağaç', 'İl', 'x2', 'ıI', 'Tür'];
const BRACKETED = ['Tapu alanı', 'Ada no', ' Kat ', 'a,b', 'Yok değer'];
const FIELDS = [...WORDS, ...BRACKETED.map((f) => f.trim())];

/** Turkish letters in their order's neighbourhood, digits and a little punctuation. */
const ALPHABET = [...'aAbBcCçÇdDeEfFgGğĞhHıIiİjJkKlLmMnNoOöÖpPrRsSşŞtTuUüÜvVyYzZ0123456789 .-_/'];
const TEXTS = ['Arsa', 'arsa', 'ARSA', 'Bahçe', 'bahçe', 'Çınar', 'çınar', 'İzmir', 'izmir', 'Işık', 'ışık', 'Iğdır', 'Şişli', 'ÖZEL', 'özel', 'Üsküdar', 'a b', ' Arsa ', 'Arsa ', 'a\\b', '{kenar}', '%%d', '^ 2', 'é', 'e', 'E', '—', '\u00a0x', 'ΟΔΟΣ', 'Ωmega', 'ǿ', 'x\u200by', 'kısa', 'uzun', ''];
const NUMERIC = ['12', '598.50', '-0', '0', '1e3', ' 7 ', '0x10', '1,5', '+.5', '5.', '-12.75', '1e400', '-1e400', '0.1', '1234567890.125', '3.14159265358979', '1e-7', '1e21', '.', '-', '12abc', '99.995', '1.005', '2.5', '-2.5', '0.000001234', '100', '9', '10'];
const LAYERS: Record<string, string> = { a: 'Parsel sınırı', b: 'Yol ekseni', c: 'İşaret' };
export const layerName = (id: string) => LAYERS[id] ?? id;

const VARIABLES = ['alan', 'ALAN', 'uzunluk', 'çevre', 'CEVRE', 'length', 'perimeter', 'köşe', 'vertices', 'tür', 'type', 'katman', 'layer', 'etiket', 'label', 'y', 'x', 'sıra', 'row_number', 'id', 'ölçek', 'scale', 'nope'];
const FUNCTIONS: readonly (readonly [string, number, number])[] = [
  ['yuvarla', 1, 2],
  ['ROUND', 1, 2],
  ['metin', 1, 2],
  ['format_number', 1, 2],
  ['sayı', 1, 1],
  ['to_real', 1, 1],
  ['tamsayı', 1, 1],
  ['int', 1, 1],
  ['mutlak', 1, 1],
  ['min', 1, 4],
  ['en_çok', 1, 4],
  ['max', 1, 4],
  ['büyük', 1, 1],
  ['upper', 1, 1],
  ['küçük', 1, 1],
  ['LOWER', 1, 1],
  ['kırp', 1, 1],
  ['uzunluk', 1, 1],
  ['len', 1, 1],
  ['parça', 2, 3],
  ['substr', 2, 3],
  ['doldur', 2, 3],
  ['lpad', 2, 3],
  ['değiştir', 3, 3],
  ['replace', 3, 3],
  ['içerir', 2, 2],
  ['başlar', 2, 2],
  ['biter', 2, 2],
  ['eğer', 3, 3],
  ['IF', 3, 3],
  ['boş', 1, 1],
  ['is_null', 1, 1],
  ['varsayılan', 2, 4],
  ['coalesce', 2, 4],
  ['foo', 1, 1],
];
const BINARY = [' + ', ' - ', ' * ', ' / ', ' % ', ' || ', ' = ', ' == ', ' != ', ' <> ', ' < ', ' <= ', ' > ', ' >= ', ' ve ', ' and ', ' veya ', ' or ', ' VE ', ' Veya ', '+', '*', '<', '||', '-'];
const NUMBERS = ['12', '0.5', '.5', '1e3', '1.5e-2', '598.50', '0', '1234567890.125', '3', '7', '100', '1e21', '2.5', '99.995', '1.005', '0.1', '1e-7', '4', '2', '1e400', '12.'];

const word = (g: Gen) => Array.from({ length: g.int(0, 6) }, () => g.pick(ALPHABET)).join('');

function text(g: Gen): string {
  const r = g.int(0, 3);
  return r === 0 ? g.pick(TEXTS) : r === 1 ? g.pick(NUMERIC) : word(g);
}

function quoted(g: Gen, s: string): string {
  const q = g.chance(0.5) ? "'" : '"';
  return q + s.split(q).join(q + q) + q;
}

function atom(g: Gen, depth: number): string {
  switch (g.int(0, 10)) {
    case 0:
    case 1:
      return g.pick(NUMBERS);
    case 2:
      return quoted(g, text(g));
    case 3:
      return g.pick(WORDS);
    case 4:
      return `[${g.pick(BRACKETED)}]`;
    case 5:
      return `$${g.pick(VARIABLES)}`;
    case 6:
      return g.pick(['doğru', 'yanlış', 'boş', 'true', 'false', 'null', 'DOĞRU', 'Boş', 'NULL']);
    case 7:
    case 8: {
      const [name, lo, hi] = g.pick(FUNCTIONS);
      // Mostly within the function's arity, now and then one too few or too many.
      const n = g.chance(0.1) ? g.pick([lo - 1, hi + 1]) : g.int(lo, hi);
      const args = Array.from({ length: Math.max(0, n) }, () => expr(g, depth + 1));
      return `${name}(${args.join(g.pick([', ', ',']))})`;
    }
    case 9:
      return `(${expr(g, depth + 1)})`;
    default:
      return g.pick(['-', '+', 'değil ', 'not ', 'NOT ', '- ']) + atom(g, depth + 1);
  }
}

function expr(g: Gen, depth: number): string {
  if (depth > 3 || g.chance(0.45)) return atom(g, depth);
  return expr(g, depth + 1) + g.pick(BINARY) + expr(g, depth + 1);
}

/** A source: an expression, now and then cut short or with a stray character in it. */
export function randomSource(g: Gen): string {
  let s = expr(g, 0);
  if (g.chance(0.12)) s = s.slice(0, g.int(0, s.length));
  if (g.chance(0.1)) {
    const i = g.int(0, s.length);
    s = s.slice(0, i) + g.pick(['#', '@', '[', ']', '$', "'", '"', '(', ')', ',', ' ', '\t', '.', '1.', 'e', ' ', '!', 've', '\n']) + s.slice(i);
  }
  return s;
}

/** A random object: a kind, attributes, a label and a layer. */
export function randomObject(g: Gen, id: number): Entity {
  const attrs: Record<string, string> = {};
  for (const f of FIELDS) if (f !== 'Yok' && f !== 'Yok değer' && g.chance(0.6)) attrs[f] = text(g);
  const base = { id, layerId: g.pick(['a', 'b', 'c', 'd']), attrs, ...(g.chance(0.5) ? { label: text(g) } : {}) };
  const p = () => ({ x: g.num(-100, 100), y: g.num(-100, 100) });
  switch (g.int(0, 5)) {
    case 0:
      return { ...base, kind: 'polygon', pts: [p(), p(), p()], ...(g.chance(0.3) ? { holes: [{ pts: [p(), p(), p()] }] } : {}) };
    case 1:
      return { ...base, kind: 'polyline', pts: Array.from({ length: g.int(0, 4) }, p) };
    case 2:
      return { ...base, kind: 'line', a: p(), b: p() };
    case 3:
      return { ...base, kind: 'point', p: p() };
    case 4:
      return { ...base, kind: 'circle', c: p(), r: g.num(1, 10) };
    default:
      return { ...base, kind: 'spline', pts: [p(), p(), p()], closed: false };
  }
}

/** Geometry values as the store gives them (six numbers an object), some missing. */
export function randomMeasures(g: Gen, n: number): Float64Array {
  const out = new Float64Array(n * MEASURE_STRIDE);
  const value = () => g.pick([0, -0, 12.5, 598.5, 1 / 3, 1e21, 1e-7, 100, -5, 123456.789, 0.1 + 0.2, 2.5, g.num(-1e4, 1e4)]);
  for (let i = 0; i < n; i++) {
    const k = i * MEASURE_STRIDE;
    out[k] = (g.chance(0.8) ? 1 : 0) + (g.chance(0.6) ? 2 : 0) + (g.chance(0.9) ? 4 : 0);
    for (let j = 1; j < 5; j++) out[k + j] = value();
  }
  return out;
}
