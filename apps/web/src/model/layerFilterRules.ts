import type { LayerFilter } from '../contracts/generated/LayerFilter';
import type { LayerNode } from './layers';

/**
 * Katman süzgeci's rules (docs/adr/0211 §2), the web's copy of `kentos_contracts::layer_filter`: the readers', the
 * commands' and the server's words. Whether a filter's condition compiles is the command's (`model/layerFilter.ts`).
 */

/** The longest condition, in characters. */
export const FILTER_EXPRESSION_MAX = 10_000;
/** The most objects a list names. */
export const FILTER_OBJECTS_MAX = 100_000;

/** What is wrong with a filter, when anything is (`LayerFilter::problem`). */
export function layerFilterProblem(f: LayerFilter): string | null {
  const objects = f.objects ?? [];
  if (f.expression == null && !objects.length) return 'süzgeçte ne ifade ne nesne listesi var';
  if (f.expression != null) {
    const e = f.expression;
    if (e.trim() !== e || !e) return 'süzgecin ifadesi boş ya da başında veya sonunda boşluk var';
    if ([...e].length > FILTER_EXPRESSION_MAX) return `süzgecin ifadesi ${FILTER_EXPRESSION_MAX} karakterden uzun`;
  }
  if (objects.length > FILTER_OBJECTS_MAX) return `süzgecin listesinde ${objects.length} nesne var; en çok ${FILTER_OBJECTS_MAX}`;
  if (objects.some((id) => id === '00000000-0000-0000-0000-000000000000')) return 'süzgecin listesinde boş (sıfır) kimlik var';
  const seen = new Set<string>();
  for (const id of objects) {
    if (seen.has(id)) return `süzgecin listesinde ${id} iki kez var`;
    seen.add(id);
  }
  return null;
}

/** What is wrong with the tree's filters, when anything is (`filters_problem`): on a group, on a service layer, broken. */
export function filtersProblem(tree: readonly LayerNode[]): string | null {
  for (const n of tree) {
    if (n.filter) {
      if (n.type === 'group') return `“${n.name}” bir grup; grubun süzgeci olmaz, süzgeç katmanındır`;
      if (n.service) return `“${n.name}” servisten çizilir; nesnesi olmayan katmanın süzgeci olmaz`;
      const p = layerFilterProblem(n.filter);
      if (p) return `“${n.name}” katmanının süzgeci: ${p}`;
    }
    const p = filtersProblem(n.children);
    if (p) return p;
  }
  return null;
}
