import type { LayerFilter } from '../contracts/generated/LayerFilter';
import type { CadDocument } from './document';
import type { Entity } from './entities';
import { IdMarks } from './idMarks';
import { compileExpression, expressionError, type CompiledExpression, type ExprGeometry } from './expression/expression';

/**
 * Katman süzgeci (docs/adr/0211 §3): a layer's filter compiled and asked over its objects, as İfadeyle seç asks a
 * condition: an object passes when the condition is true for it (false, empty or an error: it does not) and, with a
 * list, when the list names its persistent id. The desktop's is `kentos_native_application::layer_filter`; the
 * shared command cases (`cad.layers.filter`) hold the two together.
 */
export interface CompiledFilter {
  readonly expression: CompiledExpression | null;
  /** The list's persistent ids as given; null without a list. */
  readonly listed: readonly string[] | null;
  /** The same as a set, made when first asked (an object just put is looked up in it). */
  readonly objects: ReadonlySet<string> | null;
}

export type FilterCompile = { ok: true; filter: CompiledFilter } | { ok: false; error: string };

/** A filter compiled; the condition's error as the dialog says it (“12. karakterde: …”), `$sıra` and `$ölçek` refused. */
export function compileFilter(f: LayerFilter): FilterCompile {
  let expression: CompiledExpression | null = null;
  if (f.expression != null) {
    const r = compileExpression(f.expression);
    if (!r.ok) return { ok: false, error: expressionError(r) };
    if (r.expr.needs.index) return { ok: false, error: 'Süzgeçte $sıra kullanılamaz: süzgeç çalıştırmaya göre değişmemeli.' };
    if (r.expr.needs.scale) return { ok: false, error: 'Süzgeçte $ölçek kullanılamaz: süzgeç çizimin ölçeğine göre değişmemeli.' };
    expression = r.expr;
  }
  const listed = f.objects?.length ? f.objects : null;
  let set: Set<string> | null = null;
  return {
    ok: true,
    filter: {
      expression,
      listed,
      get objects() {
        return listed ? (set ??= new Set(listed)) : null;
      },
    },
  };
}

/** An object's persistent id as the document keeps it on the object (docs/adr/0014): no lookup. */
export const entityUid = (e: Entity): string | undefined => (e as { readonly uid?: string }).uid;

/**
 * For each object, whether it passes: the list by the object's persistent id first, then the condition, asked only
 * of the objects the list let through. `layerName` names a layer (`$katman`), the geometry store (when given) gives
 * the shapes its geometry values are read from.
 */
export function filterPasses(f: CompiledFilter, list: readonly Entity[], layerName: (id: string) => string, geometry?: ExprGeometry): boolean[] {
  const out = f.objects ? list.map((e) => f.objects!.has(entityUid(e) ?? '')) : list.map(() => true);
  if (!f.expression) return out;
  const asked = f.objects ? list.filter((_, i) => out[i]) : list;
  const met = filterCondition(f, asked, layerName, geometry);
  let k = 0;
  for (let i = 0; i < list.length; i++) if (out[i]) out[i] = met[k++];
  return out;
}

/**
 * For each of `doc`'s objects in `list`, whether it passes (docs/adr/0211 §6): the list is turned into the objects' ids
 * once (the document's index of persistent ids), the condition asked only of the objects it names. A whole layer's
 * way: no persistent id is looked up per object.
 */
export function filterPassesIn(doc: CadDocument, f: CompiledFilter, list: readonly Entity[], layerName: (id: string) => string, geometry?: ExprGeometry): boolean[] {
  let listed: IdMarks | null = null;
  if (f.listed) {
    listed = new IdMarks();
    for (const u of f.listed) {
      const id = doc.slotOf(u);
      if (id !== undefined) listed.add(id);
    }
  }
  const out = new Array<boolean>(list.length);
  const asked: Entity[] = [];
  const at: number[] = [];
  for (let i = 0; i < list.length; i++) {
    const inList = !listed || listed.has(list[i].id);
    out[i] = inList && !f.expression;
    if (inList && f.expression) {
      asked.push(list[i]);
      at.push(i);
    }
  }
  if (asked.length) {
    const met = filterCondition(f, asked, layerName, geometry);
    for (let k = 0; k < asked.length; k++) out[at[k]] = met[k];
  }
  return out;
}

/** For each object, whether the condition holds for it, the list aside (all of them without a condition). */
export function filterCondition(f: CompiledFilter, list: readonly Entity[], layerName: (id: string) => string, geometry?: ExprGeometry): boolean[] {
  if (!f.expression) return list.map(() => true);
  if (!list.length) return [];
  const met = f.expression.evaluateAll({ entities: list, layerName, geometry }, 'bool');
  const out = new Array<boolean>(list.length);
  for (let i = 0; i < list.length; i++) out[i] = met.value(i) === true;
  return out;
}

/**
 * The objects of `doc` their layers' filters leave out (docs/adr/0211 §1): what İşlemler does not see, asked of the
 * document itself (a run's copy has no geometry store). A layer whose condition does not compile leaves out all its
 * objects. The desktop's `layer_filter::left_out`.
 */
export function leftOut(doc: CadDocument): Set<number> {
  const out = new Set<number>();
  for (const node of doc.layers.leaves()) {
    if (!node.filter || node.service) continue;
    const list = doc.byLayer(node.id);
    const r = compileFilter(node.filter);
    const pass = r.ok ? filterPassesIn(doc, r.filter, list, (id) => doc.layers.get(id)?.name ?? id) : list.map(() => false);
    list.forEach((e, i) => {
      if (!pass[i]) out.add(e.id);
    });
  }
  return out;
}
