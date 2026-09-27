import { ENTITY_KIND_LABEL, type Entity } from '../entities';
import type { ExprField } from './builder';
import { compileExpression, expressionError, type CompileResult, type ExprGeometry, type ExprObjects } from './expression';
import type { ExprValue } from './expressionLib';

/** Today's text attributes as the builder's fields, with how many objects have each. */
export function attributeFields(fields: readonly { readonly name: string; readonly count?: number }[]): ExprField[] {
  return fields.map((f) => ({ name: f.name, type: 'text', source: 'attribute', description: f.count !== undefined ? `${f.count} nesnede var.` : undefined }));
}

/**
 * What the expression builder previews on (docs/adr/0100 §5): the objects
 * the expression will run on, one at a time, and a field's values. The
 * processing dialog, the style windows and anything else with an
 * expression field give the builder their own objects this way.
 */
export interface BuilderObjects {
  readonly count: number;
  /** The expression's value on object `i` (from 0), or why it has none. */
  value(source: string, i: number): { readonly value: ExprValue } | { readonly error: string };
  /** A field's distinct values in object order, at most `limit` of them when given. */
  values(field: string, limit?: number): string[];
  /** How the builder's stepper names object `i`: “Kapalı alan 12”. */
  describe(i: number): string;
}

/** Where the objects' geometry values come from (as `ExprObjects` takes them). */
export interface BuilderGeometry {
  /** The geometry store the objects are in: every `$` value, the centroid and box too. */
  readonly geometry?: ExprGeometry;
  /** The store's `measures` for a list of objects (`$alan`, `$uzunluk`, `$y`, `$x`). */
  readonly measures?: (list: readonly Entity[]) => Float64Array;
  /** Denominator of the plot scale ($ölçek), where there is one. */
  readonly plotScale?: number;
}

/**
 * Objects for the builder from a list of entities in run order. Object `i`
 * is evaluated as the run would: its `$sıra` is `i + 1`, so the objects
 * before it are in the call (the column engine takes them in one go).
 */
export function entityObjects(entities: readonly Entity[], layerName: (id: string) => string, g: BuilderGeometry = {}): BuilderObjects {
  let last: { source: string; result: CompileResult } | null = null;
  const compiled = (source: string) => {
    if (last?.source !== source) last = { source, result: compileExpression(source) };
    return last.result;
  };
  return {
    count: entities.length,
    value(source, i) {
      const r = compiled(source);
      if (!r.ok) return { error: expressionError(r) };
      const list = entities.slice(0, i + 1);
      const m = g.measures;
      const objects: ExprObjects = { entities: list, layerName, plotScale: g.plotScale, geometry: g.geometry, measures: m && (() => m(list)) };
      return { value: r.expr.evaluateAll(objects).value(i) };
    },
    values(field, limit) {
      const seen = new Set<string>();
      for (const e of entities) {
        if (!Object.hasOwn(e.attrs, field)) continue;
        seen.add(e.attrs[field]);
        if (limit !== undefined && seen.size >= limit) break;
      }
      return [...seen];
    },
    describe(i) {
      const e = entities[i];
      return e ? `${ENTITY_KIND_LABEL[e.kind]}${e.label ? ` ${e.label}` : ''}` : '';
    },
  };
}
