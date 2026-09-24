import type { Entity } from '../model/entities';
import { toNumber, toText, truthy, type Measured } from '../model/expression/expressionLib';
import type { ExprCache } from './compile';
import type { LayerRenderer, Rule, Symbol, SymbolRef, SymbolSet } from '../model/style';

/**
 * Which symbols an object gets from its layer's renderer. A rule-based
 * renderer can match several rules (each draws, as in QGIS); every match
 * carries the scale range it is drawn in, so the GPU can switch it per
 * frame without rebuilding.
 */

export interface ResolvedSet {
  readonly symbols: SymbolSet;
  /** 1:N range the symbols are drawn in (absent = always). */
  readonly minScale?: number;
  readonly maxScale?: number;
}

export interface ResolveEnv {
  readonly exprs: ExprCache;
  layerName(id: string): string;
  /** The geometry values of the object at 1-based `index` (the geometry store, while drawing a layer). */
  readonly measured?: (index: number) => Measured;
}

function value(src: string, e: Entity, index: number, env: ResolveEnv) {
  const x = env.exprs.get(src);
  const m = env.measured;
  return x ? x.evaluate({ entity: e, index, layerName: env.layerName, measured: m && (() => m(index)) }) : null;
}

const narrow = (a: { minScale?: number; maxScale?: number }, r: Rule) => ({
  minScale: r.minScale !== undefined ? Math.max(a.minScale ?? 0, r.minScale) : a.minScale,
  maxScale: r.maxScale !== undefined ? Math.min(a.maxScale ?? Infinity, r.maxScale) : a.maxScale,
});

function matchRules(rules: readonly Rule[], e: Entity, index: number, env: ResolveEnv, range: { minScale?: number; maxScale?: number }, out: ResolvedSet[]): boolean {
  let any = false;
  const visit = (r: Rule) => {
    const scale = narrow(range, r);
    if (scale.minScale !== undefined && scale.maxScale !== undefined && scale.minScale > scale.maxScale) return;
    if (r.symbols) out.push({ symbols: r.symbols, ...scale });
    if (r.children?.length) matchRules(r.children, e, index, env, scale, out);
  };
  for (const r of rules) {
    if (r.enabled === false || r.isElse) continue;
    if (r.filter && !truthy(value(r.filter, e, index, env))) continue;
    any = true;
    visit(r);
  }
  if (!any) for (const r of rules) if (r.enabled !== false && r.isElse) visit(r);
  return any;
}

export function resolveRenderer(renderer: LayerRenderer, e: Entity, index: number, env: ResolveEnv): ResolvedSet[] {
  switch (renderer.type) {
    case 'single':
      return [{ symbols: renderer.symbols }];
    case 'categorized': {
      const v = toText(value(renderer.expr, e, index, env));
      const c = renderer.categories.find((x) => x.enabled !== false && x.value === v);
      if (c) return [{ symbols: c.symbols }];
      return renderer.other ? [{ symbols: renderer.other }] : [];
    }
    case 'graduated': {
      const n = toNumber(value(renderer.expr, e, index, env));
      if (n === null) return [];
      const last = renderer.classes.length - 1;
      const c = renderer.classes.find((k, i) => n >= k.min && (n < k.max || (i === last && n <= k.max)));
      return c ? [{ symbols: c.symbols }] : [];
    }
    case 'rules': {
      const out: ResolvedSet[] = [];
      matchRules(renderer.rules, e, index, env, {}, out);
      return out;
    }
  }
}

/** A library reference or an inline symbol, looked up; null when the library has no such symbol. */
export function symbolOf(ref: SymbolRef | undefined, lookup: (id: string) => Symbol | undefined): Symbol | null {
  if (!ref) return null;
  return 'ref' in ref ? (lookup(ref.ref) ?? null) : ref;
}
