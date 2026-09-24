import type { Entity } from '../model/entities';
import { compileExpression, type ExprAs } from '../model/expression/expression';
import type { Color, Symbol, SymbolSet } from '../model/style';
import { geometryClassOf, type GeometryClass } from './geometry';

/**
 * Classes for the layer style window (QGIS "Classify"): the distinct
 * values of an expression over a layer's objects, numeric classes by equal
 * interval or equal count, colour ramps, and the plain symbols a new class
 * starts with. Pure and testable; the window only shows and edits them.
 */

export interface ValueCount {
  value: string;
  count: number;
}

type Scope = {
  layerName(id: string): string;
  /** The geometry store's values of objects (`measuredAt` records), asked once for all of them when the expression first needs `$alan`, `$uzunluk`, `$y` or `$x` (docs/adr/0008). */
  measures?(entities: readonly Entity[]): Float64Array;
};

/** The expression's value per object, as `as` asks, in one call to the core; or an error. */
function evaluated<T>(entities: readonly Entity[], expr: string, scope: Scope, as: ExprAs): { values: T[]; error?: string } {
  const c = compileExpression(expr);
  if (!c.ok) return { values: [], error: c.error };
  const measures = scope.measures;
  const col = c.expr.evaluateAll({ entities, layerName: scope.layerName, measures: measures && (() => measures(entities)) }, as);
  return { values: entities.map((_, i) => col.value(i) as T) };
}

/** The expression's text per object (null when it gives nothing), or an error. */
export const valuesOf = (entities: readonly Entity[], expr: string, scope: Scope) => evaluated<string | null>(entities, expr, scope, 'text');

/** The numbers the expression's values read as (text that reads as a number counts), or an error. */
export function numbersOf(entities: readonly Entity[], expr: string, scope: Scope): { values: number[]; error?: string } {
  const r = evaluated<number | null>(entities, expr, scope, 'textNumber');
  return { values: r.values.filter((v): v is number => v !== null && Number.isFinite(v)), error: r.error };
}

/** Distinct non-empty values with their counts, ordered naturally (numbers by value, text in Turkish order). */
export function uniqueValues(values: readonly (string | null)[]): ValueCount[] {
  const counts = new Map<string, number>();
  for (const v of values) if (v !== null && v !== '') counts.set(v, (counts.get(v) ?? 0) + 1);
  return [...counts.entries()].map(([value, count]) => ({ value, count })).sort((a, b) => a.value.localeCompare(b.value, 'tr', { numeric: true }));
}

export interface NumericClass {
  min: number;
  max: number;
}

/** `n` classes of equal width between the smallest and largest value. */
export function equalInterval(values: readonly number[], n: number): NumericClass[] {
  if (!values.length || n < 1) return [];
  const lo = Math.min(...values);
  const hi = Math.max(...values);
  if (hi === lo) return [{ min: lo, max: hi }];
  const step = (hi - lo) / n;
  return Array.from({ length: n }, (_, i) => ({ min: lo + i * step, max: i === n - 1 ? hi : lo + (i + 1) * step }));
}

/** `n` classes with about the same number of objects each (quantiles); equal bounds merge. */
export function equalCount(values: readonly number[], n: number): NumericClass[] {
  if (!values.length || n < 1) return [];
  const sorted = [...values].sort((a, b) => a - b);
  const bounds = [sorted[0]];
  for (let i = 1; i < n; i++) bounds.push(sorted[Math.min(sorted.length - 1, Math.round((i * sorted.length) / n))]);
  bounds.push(sorted[sorted.length - 1]);
  const out: NumericClass[] = [];
  for (let i = 0; i < bounds.length - 1; i++) if (bounds[i + 1] > bounds[i] || (i === bounds.length - 2 && !out.length)) out.push({ min: bounds[i], max: bounds[i + 1] });
  return out;
}

// ── Colours ────────────────────────────────────────────────────────────

/** Well-separated colours for categories (QGIS-like "rastgele" but repeatable). */
export const QUALITATIVE: readonly Color[] = ['#E15759', '#4E79A7', '#F28E2B', '#76B7B2', '#59A14F', '#EDC948', '#B07AA1', '#FF9DA7', '#9C755F', '#BAB0AC', '#8CD17D', '#86BCB6'];

export const RAMPS: Record<string, { label: string; stops: readonly Color[] }> = {
  sariKirmizi: { label: 'Sarıdan kırmızıya', stops: ['#FFF5B8', '#FDB863', '#E66101', '#A50F15'] },
  maviler: { label: 'Maviler', stops: ['#E3EEF9', '#9ECAE1', '#4292C6', '#08306B'] },
  yesiller: { label: 'Yeşiller', stops: ['#E5F5E0', '#A1D99B', '#41AB5D', '#005A32'] },
  griler: { label: 'Griler', stops: ['#F0F0F0', '#BDBDBD', '#737373', '#252525'] },
  spektral: { label: 'Spektral', stops: ['#2B83BA', '#ABDDA4', '#FFFFBF', '#FDAE61', '#D7191C'] },
};

const hex = (c: Color) => [1, 3, 5].map((i) => parseInt(c.slice(i, i + 2), 16));
const toHex = (rgb: number[]) => `#${rgb.map((v) => Math.round(v).toString(16).padStart(2, '0')).join('').toUpperCase()}`;

/** `n` colours along a ramp, ends included. */
export function rampColors(stops: readonly Color[], n: number): Color[] {
  if (n <= 1) return [stops[stops.length - 1]];
  return Array.from({ length: n }, (_, i) => {
    const t = (i / (n - 1)) * (stops.length - 1);
    const k = Math.min(stops.length - 2, Math.floor(t));
    const a = hex(stops[k]);
    const b = hex(stops[k + 1]);
    const f = t - k;
    return toHex(a.map((v, j) => v + (b[j] - v) * f));
  });
}

// ── Symbols for new classes ────────────────────────────────────────────

/** The geometry classes a layer's objects have (text and dimensions have none). */
export function classesPresent(entities: readonly Entity[]): Record<GeometryClass, number> {
  const out: Record<GeometryClass, number> = { fill: 0, line: 0, marker: 0 };
  for (const e of entities) {
    const c = geometryClassOf(e);
    if (c) out[c]++;
  }
  return out;
}

/** A plain symbol of one colour for each geometry class the layer has (areas get a thin ink edge). */
export function plainSymbols(color: Color, present: Record<GeometryClass, number>): SymbolSet {
  const fill: Symbol = { type: 'fill', layers: [{ id: '0', type: 'simpleFill', color }, { id: '1', type: 'simpleLine', color: 'ink', width: 0.18 }] };
  const line: Symbol = { type: 'line', layers: [{ id: '0', type: 'simpleLine', color, width: 0.35 }] };
  const marker: Symbol = { type: 'marker', layers: [{ id: '0', type: 'shape', shape: 'circle', size: 2.4, fill: color, stroke: 'ink', strokeWidth: 0.18 }] };
  const any = present.fill + present.line + present.marker > 0;
  return {
    ...(present.fill || !any ? { fill } : {}),
    ...(present.line || !any ? { line } : {}),
    ...(present.marker || !any ? { marker } : {}),
  };
}

/** Label for a numeric class: "12.5 – 30". */
export const classLabel = (c: NumericClass, digits = 2) => `${round(c.min, digits)} – ${round(c.max, digits)}`;
const round = (v: number, d: number) => String(Math.round(v * 10 ** d) / 10 ** d);
