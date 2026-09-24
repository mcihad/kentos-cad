import type { Entity } from '../model/entities';
import type { ExprRun } from './compile';
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
  /** The build's expressions, evaluated for all its objects at once; `index` is a 1-based position in them. */
  readonly run: ExprRun;
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
    if (r.filter && env.run.value(r.filter, index, 'bool') !== true) continue;
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
      const v = (env.run.value(renderer.expr, index, 'text') as string | null) ?? '';
      const c = renderer.categories.find((x) => x.enabled !== false && x.value === v);
      if (c) return [{ symbols: c.symbols }];
      return renderer.other ? [{ symbols: renderer.other }] : [];
    }
    case 'graduated': {
      const n = env.run.value(renderer.expr, index, 'number') as number | null;
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
