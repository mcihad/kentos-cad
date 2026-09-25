import { exprEvaluate, op, type ExprColumnData } from '../../wasm/core';
import { ENTITY_KIND_LABEL, entityAnchor, entityArea, entityLength, type Entity } from '../entities';
import { MEASURE_STRIDE, type ExprValue } from './expressionLib';

/**
 * İfadeler: a small, safe expression language for processing tools
 * (select by expression, field calculator, filters) and the style engine
 * (data-defined values, rule and category renderers).
 *
 *   Nitelik = 'Arsa' ve $alan > 500
 *   'P' || doldur($sıra, 5)
 *   yuvarla([Tapu alanı] - $alan, 2)
 *
 * The language is the Rust style core's (crates/shared/style-core/src/expr,
 * docs/adr/0008 “İfade dili”): the server evaluates the same expressions
 * the same way. This module builds the table of what an expression reads
 * of each object and reads the answer back; an expression is evaluated for
 * a whole list of objects in one call.
 */

export type { ExprValue };

/** The variables an expression reads, so only those are computed. */
export interface ExprNeeds {
  readonly measured: boolean;
  readonly vertices: boolean;
  readonly kind: boolean;
  readonly layer: boolean;
  readonly label: boolean;
  readonly index: boolean;
  readonly id: boolean;
  readonly scale: boolean;
}

/** What an expression is evaluated on: objects in run order ($sıra is the position + 1). */
export interface ExprObjects {
  readonly entities: readonly Entity[];
  layerName(id: string): string;
  /** Denominator of the plot scale while drawing a symbol ($ölçek); absent elsewhere. */
  readonly plotScale?: number;
  /** The geometry store's `measures` answer for these objects (six numbers each); computed per object when absent. */
  readonly measures?: () => Float64Array;
}

/**
 * What the caller wants of each value: as it is, a number (empty when it is
 * not one), text, true/false (empty stays empty for these two), or the number
 * the value's text reads as (the style window's classes).
 */
export type ExprAs = 'value' | 'number' | 'text' | 'bool' | 'textNumber';

/** An expression's values for a list of objects. */
export interface ExprColumn {
  readonly length: number;
  value(i: number): ExprValue;
}

export interface CompiledExpression {
  readonly source: string;
  /** Attribute names the expression reads (to warn about missing ones). */
  readonly fields: readonly string[];
  readonly needs: ExprNeeds;
  /** The value for each object, as `as` asks, in one call to the core. */
  evaluateAll(objects: ExprObjects, as?: ExprAs): ExprColumn;
}

export type CompileResult = { ok: true; expr: CompiledExpression } | { ok: false; error: string; at: number };

type Compiled = { ok: true; fields: string[]; needs: ExprNeeds } | { ok: false; error: string; at: number };
const exprCompile = op<(source: string) => Compiled>('exprCompile');

export function compileExpression(source: string): CompileResult {
  const r = exprCompile(source);
  if (!r.ok) return { ok: false, error: r.error, at: r.at };
  const { fields, needs } = r;
  return { ok: true, expr: { source, fields, needs, evaluateAll: (objects, as = 'value') => evaluateAll(source, fields, needs, objects, as) } };
}

/** Message for the dialog: "12. karakterde: …". */
export const expressionError = (r: Extract<CompileResult, { ok: false }>) => (r.at > 1 ? `${r.at}. karakterde: ${r.error}` : r.error);

/** Corners of a path or area (holes included), for $köşe. */
export function vertexCount(e: Entity): number | null {
  switch (e.kind) {
    case 'polyline':
      return e.pts.length;
    case 'polygon': {
      let n = e.pts.length;
      for (const h of e.holes ?? []) n += h.pts.length;
      return n;
    }
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

/** The geometry values of objects without a geometry store (tests, symbol previews): the core's measures, one object at a time. */
function measuresOf(list: readonly Entity[]): Float64Array {
  const out = new Float64Array(list.length * MEASURE_STRIDE);
  list.forEach((e, i) => {
    const k = i * MEASURE_STRIDE;
    const length = entityLength(e);
    const area = entityArea(e);
    const at = entityAnchor(e);
    // Flags: 1 length, 2 area, 4 anchor (crates/shared/style-core/src/expr/rows.rs).
    out[k] = (length !== null ? 1 : 0) | (area !== null ? 2 : 0) | (at ? 4 : 0);
    out[k + 1] = length ?? 0;
    out[k + 2] = area ?? 0;
    out[k + 3] = at?.x ?? 0;
    out[k + 4] = at?.y ?? 0;
  });
  return out;
}

const WANT: Record<ExprAs, number> = { value: 0, number: 1, text: 2, bool: 3, textNumber: 4 };
const NO_NUMBERS = new Float64Array(0);

/** What expressions read of each object, as the core takes it (`exprTable`). */
export interface ExprTable {
  readonly texts: string;
  readonly lens: Int32Array;
  readonly numbers: Float64Array;
}

/**
 * The table of what expressions read (crates/shared/style-core/src/expr/rows.rs
 * has the layout): text slots per object (the fields in order, then the label,
 * the layer name and the kind label, each only when read; a missing value is
 * length −1), number slots (the id, then the vertex count), one object after
 * another. A styled layer's table serves all its expressions (`fields` their union).
 */
export function exprTable(fields: readonly string[], needs: ExprNeeds, list: readonly Entity[], layerName: (id: string) => string): ExprTable {
  // Text slots per object.
  const n = fields.length + [needs.label, needs.layer, needs.kind].filter(Boolean).length;
  // One text, joined as it goes (faster than joining a list of 100 000 at the end).
  let texts = '';
  const lens = new Int32Array(list.length * n);
  const numbers = new Float64Array(list.length * [needs.id, needs.vertices].filter(Boolean).length);
  let j = 0;
  let k = 0;
  const put = (v: string | null) => {
    if (v === null) lens[j++] = -1;
    else {
      texts += v;
      lens[j++] = v.length;
    }
  };
  for (const e of list) {
    // Own attributes only: a field named "constructor" is not the object's prototype.
    for (const f of fields) put(Object.hasOwn(e.attrs, f) ? e.attrs[f] : null);
    if (needs.label) put(e.label ?? null);
    if (needs.layer) put(layerName(e.layerId));
    if (needs.kind) put(ENTITY_KIND_LABEL[e.kind]);
    if (needs.id) numbers[k++] = e.id;
    if (needs.vertices) numbers[k++] = vertexCount(e) ?? NaN;
  }
  return { texts, lens, numbers };
}

function evaluateAll(source: string, fields: readonly string[], needs: ExprNeeds, o: ExprObjects, as: ExprAs): ExprColumn {
  const list = o.entities;
  const { texts, lens, numbers } = exprTable(fields, needs, list, o.layerName);
  const measures = needs.measured ? (o.measures?.() ?? measuresOf(list)) : NO_NUMBERS;
  return column(exprEvaluate(source, list.length, texts, lens, numbers, measures, o.plotScale ?? NaN, WANT[as]));
}

/** The core's answer read back: where each object's text starts, and its length, are found once. */
function column(c: ExprColumnData): ExprColumn {
  let spans: Int32Array | null = null;
  const textSpans = () => {
    const out = new Int32Array(c.kinds.length * 2);
    let from = 0;
    let j = 0;
    for (let i = 0; i < c.kinds.length; i++)
      if (c.kinds[i] === 2) {
        out[2 * i] = from;
        out[2 * i + 1] = c.textLengths[j];
        from += c.textLengths[j++];
      }
    return out;
  };
  return {
    length: c.kinds.length,
    value(i: number): ExprValue {
      switch (c.kinds[i]) {
        case 1:
          return c.numbers[i];
        case 2: {
          const s = (spans ??= textSpans());
          return c.texts.slice(s[2 * i], s[2 * i] + s[2 * i + 1]);
        }
        case 3:
          return c.numbers[i] === 1;
        default:
          return null;
      }
    },
  };
}

/**
 * One line for the dialog: how the expression works out on the objects it
 * will read. `measures`: the geometry store's values of objects, asked once
 * for all the previewed objects when the expression needs `$alan`,
 * `$uzunluk`, `$y` or `$x`.
 */
export function previewExpression(expr: CompiledExpression, entities: readonly Entity[], kind: 'condition' | 'value', layerName: (id: string) => string, measures?: (entities: readonly Entity[]) => Float64Array): string {
  if (!entities.length) return 'Önizleme için uygun nesne yok.';
  const missing = expr.fields.filter((f) => !entities.some((e) => Object.hasOwn(e.attrs, f)));
  const note = missing.length ? ` ${missing.map((f) => `“${f}”`).join(', ')} alanı bu nesnelerde yok.` : '';
  const list = entities.slice(0, 20000);
  const objects: ExprObjects = { entities: list, layerName, measures: measures && (() => measures(list)) };
  if (kind === 'condition') {
    const c = expr.evaluateAll(objects, 'bool');
    const hits = list.filter((_, i) => c.value(i) === true).length;
    return `${hits} / ${list.length} nesne koşulu sağlıyor.${note}`;
  }
  const c = expr.evaluateAll(objects, 'text');
  const first = c.value(0);
  const empty = list.filter((_, i) => c.value(i) === null).length;
  const who = entities[0].label ? ` (${entities[0].label})` : '';
  return `İlk nesnede${who}: ${first === null ? 'boş' : `“${first}”`}.${empty ? ` ${empty} nesnede sonuç boş.` : ''}${note}`;
}
