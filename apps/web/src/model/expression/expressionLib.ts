import { foldTurkish } from '../../core/text';
import { ENTITY_KIND_LABEL, entityAnchor, entityArea, entityLength, type Entity } from '../entities';
import type { Vec2 } from '../geometry';

/**
 * Values, variables and functions of the expression language
 * (expression.ts). Attribute values are text, so arithmetic reads numbers
 * out of text ("452.13" → 452.13) and an empty or missing value is null.
 * Names are matched Turkish-insensitively: yuvarla = YUVARLA, $çevre = $cevre;
 * every function also answers to its English (QGIS) name.
 */

export type ExprValue = number | string | boolean | null;

/** What an expression sees for one object. */
export interface ExprScope {
  readonly entity: Entity;
  /** 1-based position of the object in the run ($sira). */
  readonly index: number;
  layerName(id: string): string;
  /** Denominator of the plot scale while drawing a symbol ($ölçek); absent elsewhere. */
  readonly plotScale?: number;
  /** The object's geometry values from the geometry store (drawing a layer, docs/adr/0008 S2); computed here when absent. */
  readonly measured?: () => Measured;
}

/** An object's geometry values for expressions: `$uzunluk`, `$alan`, and the anchor behind `$y` and `$x`. */
export interface Measured {
  readonly length: number | null;
  readonly area: number | null;
  /** Null for a path without vertices. */
  readonly anchor: Vec2 | null;
}

/** Numbers per object in a geometry store `measures` answer: flags, length, area, anchor x and y, spare. */
const MEASURE_STRIDE = 6;

/**
 * The geometry values of object `i` in a geometry store `measures` answer
 * (crates/shared/geometry-core/src/store/draw.rs), for drawing a layer and for
 * processing runs alike (docs/adr/0008, S2 and S4).
 */
export function measuredAt(values: Float64Array, i: number): Measured {
  const k = i * MEASURE_STRIDE;
  const flags = values[k];
  return {
    length: flags & 1 ? values[k + 1] : null,
    area: flags & 2 ? values[k + 2] : null,
    anchor: flags & 4 ? { x: values[k + 3], y: values[k + 4] } : null,
  };
}

/**
 * The geometry values of a list of objects (object `i` of it), fetched from
 * the geometry store for all of them the first time an expression asks for
 * one; an expression that asks for none costs nothing.
 */
export function measuredOf(fetch: () => Float64Array): (i: number) => Measured {
  let values: Float64Array | null = null;
  return (i) => measuredAt((values ??= fetch()), i);
}

const NUMERIC = /^[+-]?(\d+(\.\d*)?|\.\d+)([eE][+-]?\d+)?$/;

/** A number, or null when the value is empty or not a number (decimal separator is the dot). */
export function toNumber(v: ExprValue): number | null {
  if (typeof v === 'number') return Number.isFinite(v) ? v : null;
  if (typeof v === 'boolean') return v ? 1 : 0;
  if (typeof v === 'string') {
    const t = v.trim();
    return NUMERIC.test(t) ? Number(t) : null;
  }
  return null;
}

export const isEmpty = (v: ExprValue) => v === null || v === '';

export function truthy(v: ExprValue): boolean {
  if (v === null) return false;
  if (typeof v === 'boolean') return v;
  if (typeof v === 'number') return v !== 0;
  return v !== '';
}

/** Text for writing into an attribute; float noise is dropped (0.1 + 0.2 → "0.3"). */
export function toText(v: ExprValue): string {
  if (v === null) return '';
  if (typeof v === 'boolean') return v ? 'doğru' : 'yanlış';
  if (typeof v === 'number') return Number.isInteger(v) ? String(v) : String(Number(v.toPrecision(12)));
  return v;
}

/** Equality: numbers by value, text exactly, and "empty" equals only "empty". */
export function equals(a: ExprValue, b: ExprValue): boolean {
  if (isEmpty(a) || isEmpty(b)) return isEmpty(a) && isEmpty(b);
  if (typeof a === 'boolean' || typeof b === 'boolean') return truthy(a) === truthy(b);
  const na = toNumber(a);
  const nb = toNumber(b);
  if (na !== null && nb !== null) return Math.abs(na - nb) <= 1e-9 * Math.max(1, Math.abs(na), Math.abs(nb));
  return toText(a) === toText(b);
}

/** Order of two values (numbers numerically, text in Turkish order); null when one is empty. */
export function compare(a: ExprValue, b: ExprValue): number | null {
  if (isEmpty(a) || isEmpty(b)) return null;
  const na = toNumber(a);
  const nb = toNumber(b);
  if (na !== null && nb !== null) return na - nb;
  return toText(a).localeCompare(toText(b), 'tr');
}

// ── Variables ──────────────────────────────────────────────────────────

export interface ExprVariable {
  /** Name without "$", as shown. */
  readonly name: string;
  readonly aliases?: readonly string[];
  readonly description: string;
  get(s: ExprScope): ExprValue;
}

function vertexCount(e: Entity): number | null {
  switch (e.kind) {
    case 'polyline':
    case 'polygon':
      return e.pts.length + (e.kind === 'polygon' ? (e.holes ?? []).reduce((n, h) => n + h.pts.length, 0) : 0);
    case 'line':
      return 2;
    case 'spline':
      return e.pts.length;
    case 'point':
      return 1;
    default:
      return null;
  }
}

export const EXPR_VARIABLES: readonly ExprVariable[] = [
  { name: 'alan', description: 'Alan (m²): kapalı alan (delikler düşülür), daire, tam elips, tarama', get: (s) => (s.measured ? s.measured().area : entityArea(s.entity)) },
  { name: 'uzunluk', aliases: ['çevre', 'length', 'perimeter'], description: 'Uzunluk ya da çevre (m)', get: (s) => (s.measured ? s.measured().length : entityLength(s.entity)) },
  { name: 'köşe', aliases: ['vertices'], description: 'Köşe sayısı (delikler dahil)', get: (s) => vertexCount(s.entity) },
  { name: 'tür', aliases: ['type'], description: 'Nesne türü: “Kapalı alan”, “Çizgi” …', get: (s) => ENTITY_KIND_LABEL[s.entity.kind] },
  { name: 'katman', aliases: ['layer'], description: 'Katman adı', get: (s) => s.layerName(s.entity.layerId) },
  { name: 'etiket', aliases: ['label'], description: 'Çizimde görünen etiket (parsel no, nokta adı)', get: (s) => s.entity.label ?? null },
  { name: 'y', description: 'Y (sağa): nesnenin yer noktası', get: (s) => (s.measured ? (s.measured().anchor?.x ?? null) : entityAnchor(s.entity).x) },
  { name: 'x', description: 'X (yukarı): nesnenin yer noktası', get: (s) => (s.measured ? (s.measured().anchor?.y ?? null) : entityAnchor(s.entity).y) },
  { name: 'sıra', aliases: ['row_number'], description: 'Bu çalıştırmadaki sırası: 1, 2, 3 …', get: (s) => s.index },
  { name: 'id', description: 'Nesne numarası', get: (s) => s.entity.id },
  { name: 'ölçek', aliases: ['scale'], description: 'Çizim ölçeğinin paydası (1/1000 için 1000); yalnızca sembol çizilirken. Metreyi kâğıt mm’sine çevirir: m × 1000 / $ölçek', get: (s) => s.plotScale ?? null },
];

// ── Functions ──────────────────────────────────────────────────────────

export interface ExprFunction {
  readonly name: string;
  /** English (QGIS) and spelling variants. */
  readonly aliases: readonly string[];
  /** Least and most arguments (Infinity: any number). */
  readonly arity: readonly [number, number];
  readonly signature: string;
  readonly description: string;
  call(args: ExprValue[]): ExprValue;
}

const num = (v: ExprValue) => toNumber(v);
const text = (v: ExprValue) => toText(v);
const fold = (v: ExprValue) => foldTurkish(toText(v));

/** Numeric function: null in, null out. */
const numeric =
  (fn: (...n: number[]) => number) =>
  (args: ExprValue[]): ExprValue => {
    const ns = args.map(num);
    if (ns.some((n) => n === null)) return null;
    const r = fn(...(ns as number[]));
    return Number.isFinite(r) ? r : null;
  };

export const EXPR_FUNCTIONS: readonly ExprFunction[] = [
  {
    name: 'yuvarla',
    aliases: ['round'],
    arity: [1, 2],
    signature: 'yuvarla(sayı, basamak)',
    description: 'Verilen ondalık basamağa yuvarlar: yuvarla(12.345, 2) → 12.35',
    call: ([x, n]) => {
      const v = num(x);
      const d = n === undefined ? 0 : num(n);
      if (v === null || d === null) return null;
      const f = 10 ** Math.max(0, Math.min(12, Math.round(d)));
      return Math.round((v + Math.sign(v) * Number.EPSILON * Math.abs(v)) * f) / f;
    },
  },
  {
    name: 'metin',
    aliases: ['to_string', 'text', 'format_number'],
    arity: [1, 2],
    signature: 'metin(değer, basamak)',
    description: 'Metne çevirir; basamak verilirse sabit ondalıkla: metin(452.1, 2) → "452.10"',
    call: ([x, n]) => {
      if (n === undefined) return text(x);
      const v = num(x);
      const d = num(n);
      if (v === null || d === null) return null;
      return v.toFixed(Math.max(0, Math.min(12, Math.round(d))));
    },
  },
  { name: 'sayı', aliases: ['to_real', 'to_number', 'number'], arity: [1, 1], signature: 'sayı(metin)', description: 'Metindeki sayıyı okur; sayı değilse boş', call: ([x]) => num(x) },
  { name: 'tamsayı', aliases: ['int', 'to_int', 'tam'], arity: [1, 1], signature: 'tamsayı(sayı)', description: 'Ondalık kısmı atar', call: numeric(Math.trunc) },
  { name: 'mutlak', aliases: ['abs'], arity: [1, 1], signature: 'mutlak(sayı)', description: 'Mutlak değer', call: numeric(Math.abs) },
  { name: 'min', aliases: ['en_az', 'enaz'], arity: [1, Infinity], signature: 'min(a, b, …)', description: 'En küçük sayı', call: numeric(Math.min) },
  { name: 'max', aliases: ['en_çok', 'encok'], arity: [1, Infinity], signature: 'max(a, b, …)', description: 'En büyük sayı', call: numeric(Math.max) },
  { name: 'büyük', aliases: ['upper'], arity: [1, 1], signature: 'büyük(metin)', description: 'Büyük harfe çevirir (Türkçe: i → İ)', call: ([s]) => (s === null ? null : text(s).toLocaleUpperCase('tr-TR')) },
  { name: 'küçük', aliases: ['lower'], arity: [1, 1], signature: 'küçük(metin)', description: 'Küçük harfe çevirir (Türkçe: I → ı)', call: ([s]) => (s === null ? null : text(s).toLocaleLowerCase('tr-TR')) },
  { name: 'kırp', aliases: ['trim'], arity: [1, 1], signature: 'kırp(metin)', description: 'Baştaki ve sondaki boşlukları siler', call: ([s]) => (s === null ? null : text(s).trim()) },
  { name: 'uzunluk', aliases: ['length', 'len'], arity: [1, 1], signature: 'uzunluk(metin)', description: 'Metnin karakter sayısı (nesne uzunluğu için $uzunluk)', call: ([s]) => (s === null ? null : text(s).length) },
  {
    name: 'parça',
    aliases: ['substr', 'parca'],
    arity: [2, 3],
    signature: 'parça(metin, başlangıç, uzunluk)',
    description: 'Metnin bir parçası; ilk karakter 1: parça("P00012", 2, 3) → "000"',
    call: ([s, a, n]) => {
      const from = num(a);
      const len = n === undefined ? Infinity : num(n);
      if (s === null || from === null || len === null) return null;
      const start = Math.max(0, Math.round(from) - 1);
      return text(s).slice(start, len === Infinity ? undefined : start + Math.max(0, Math.round(len)));
    },
  },
  {
    name: 'doldur',
    aliases: ['lpad'],
    arity: [2, 3],
    signature: 'doldur(değer, uzunluk, karakter)',
    description: 'Soldan doldurur: doldur(12, 5, "0") → "00012" (karakter verilmezse 0)',
    call: ([x, n, c]) => {
      const len = num(n);
      if (x === null || len === null) return null;
      const ch = c === undefined ? '0' : text(c).slice(0, 1) || '0';
      return text(x).padStart(Math.max(0, Math.round(len)), ch);
    },
  },
  {
    name: 'değiştir',
    aliases: ['replace', 'degistir'],
    arity: [3, 3],
    signature: 'değiştir(metin, aranan, yeni)',
    description: 'Metindeki her aranan parçayı yenisiyle değiştirir',
    call: ([s, a, b]) => (s === null ? null : text(s).split(text(a)).join(text(b))),
  },
  { name: 'içerir', aliases: ['contains', 'icerir'], arity: [2, 2], signature: 'içerir(metin, aranan)', description: 'Metin aranan parçayı içeriyor mu (büyük/küçük harf ve Türkçe harf farkı gözetilmez)', call: ([s, t]) => (s === null ? false : fold(s).includes(fold(t))) },
  { name: 'başlar', aliases: ['starts_with', 'baslar'], arity: [2, 2], signature: 'başlar(metin, aranan)', description: 'Metin aranan parçayla başlıyor mu (harf farkı gözetilmez)', call: ([s, t]) => (s === null ? false : fold(s).startsWith(fold(t))) },
  { name: 'biter', aliases: ['ends_with'], arity: [2, 2], signature: 'biter(metin, aranan)', description: 'Metin aranan parçayla bitiyor mu (harf farkı gözetilmez)', call: ([s, t]) => (s === null ? false : fold(s).endsWith(fold(t))) },
  { name: 'eğer', aliases: ['if', 'eger'], arity: [3, 3], signature: 'eğer(koşul, doğruysa, yanlışsa)', description: 'Koşula göre iki değerden birini verir', call: ([k, a, b]) => (truthy(k) ? a : b) },
  { name: 'boş', aliases: ['is_empty', 'bos', 'is_null'], arity: [1, 1], signature: 'boş(değer)', description: 'Değer boş mu (alan yok ya da boş metin)', call: ([x]) => isEmpty(x) },
  { name: 'varsayılan', aliases: ['coalesce', 'varsayilan'], arity: [2, Infinity], signature: 'varsayılan(a, b, …)', description: 'Boş olmayan ilk değer', call: (args) => args.find((a) => !isEmpty(a)) ?? null },
];

const key = (name: string) => foldTurkish(name).replace(/\s+/g, '');
const FUNCTIONS = new Map<string, ExprFunction>(EXPR_FUNCTIONS.flatMap((f) => [f.name, ...f.aliases].map((n) => [key(n), f] as const)));
const VARIABLES = new Map<string, ExprVariable>(EXPR_VARIABLES.flatMap((v) => [v.name, ...(v.aliases ?? [])].map((n) => [key(n), v] as const)));

export const findFunction = (name: string) => FUNCTIONS.get(key(name));
export const findVariable = (name: string) => VARIABLES.get(key(name));
