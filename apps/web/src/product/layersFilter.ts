import type { LayerFilter } from '../contracts/generated/LayerFilter';
import type { LayerNode as ContractLayerNode } from '../contracts/generated/LayerNode';
import type { LayersFilter } from '../contracts/generated/LayersFilter';
import type { LayersFiltered } from '../contracts/generated/LayersFiltered';
import type { LayersFilterPlan } from '../contracts/generated/LayersFilterPlan';
import { Refusal, type CadDocument } from '../model/document';
import { compileFilter, filterPassesIn, type CompiledFilter } from '../model/layerFilter';
import { layerFilterProblem } from '../model/layerFilterRules';
import { filterOf, type LayerNode } from '../model/layers';
import { sameJson } from '../model/sameJson';
import { checkRevision, error, failed, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.layers.filter` v1 (docs/adr/0211 §5): a layer's filter written, or taken away, as one undo step “Katman
 * süzgeci”; a layer that already has it as given is left as it is. The answer counts the layer's objects that pass it.
 * The web's handler over `CadDocument`; the desktop's is `crates/native/application/src/layers_filter.rs`. Both pass
 * the shared cases in fixtures/commands/v1/cad.layers.filter.json.
 *
 * The checks, in the contract's order: the filter's rules; its condition compiles (`$sıra` and `$ölçek` refused); the
 * expected revision; the layer is the drawing's, a layer, not drawn from a service.
 */

/** The undo step's name. */
export const LAYERS_FILTER_LABEL = 'Katman süzgeci';

interface Checked {
  readonly node: LayerNode;
  readonly compiled: CompiledFilter | null;
}

function check(doc: CadDocument, input: LayersFilter): Stop | Checked {
  let compiled: CompiledFilter | null = null;
  if (input.filter) {
    const problem = layerFilterProblem(input.filter);
    if (problem) return failed(error('invalid_filter', `Katmanın süzgeci: ${problem}.`, 'filter'));
    const r = compileFilter(input.filter);
    if (!r.ok) return failed(error('invalid_expression', `Süzgecin ifadesi: ${r.error}`, 'filter/expression'));
    compiled = r.filter;
  }
  const stale = checkRevision(doc, input.expectedRevision);
  if (stale) return stale;
  const id = input.layer;
  const node = doc.layers.get(id);
  if (!node) return failed(error('layer_not_found', `“${id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.`, 'layer'));
  if (node.type !== 'layer') return failed(error('not_a_layer', `“${node.name}” bir katman grubu; süzgeç yalnız katmanın olur. Grubun bir katmanını verin.`, 'layer'));
  if (node.service && input.filter)
    return failed(error('service_layer', `“${node.name}” servisten çizilir ve nesne tutmaz; süzgeç nesneleri olan katmanın olur.`, 'layer'));
  return { node, compiled };
}

const isStop = (c: Stop | Checked): c is Stop => 'status' in c;

/** How many of the layer's objects pass `filter`, and how many it has. */
export function countPassing(doc: CadDocument, layer: string, filter: CompiledFilter | null): { passed: number; total: number } {
  const list = doc.byLayer(layer);
  if (!filter) return { passed: list.length, total: list.length };
  const pass = filterPassesIn(doc, filter, list, (id) => doc.layers.get(id)?.name ?? id);
  return { passed: pass.filter(Boolean).length, total: list.length };
}

/** The input's filter as a layer keeps it; none when it is taken away. */
const filterIn = (input: LayersFilter): LayerFilter | null => (input.filter ? filterOf(input.filter) : null);

export const layersFilter: ProductCommand<LayersFilter, LayersFiltered, LayersFilterPlan> = {
  id: 'cad.layers.filter',
  version: 1,

  validate(cx, input) {
    const checked = check(cx.doc, input);
    return validated(isStop(checked) ? checked : []);
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const filter = filterIn(input);
    const { filter: _filter, ...rest } = structuredClone(checked.node);
    const node = { ...rest, ...(filter && { filter }) };
    const { passed, total } = countPassing(cx.doc, checked.node.id, checked.compiled);
    return {
      status: 'completed',
      output: { node: node as unknown as ContractLayerNode, changed: !sameJson(checked.node.filter ?? null, filter), passed, total, revision: String(cx.doc.revision) },
      warnings: [],
    };
  },

  /** Writes the filter as one undo step “Katman süzgeci” when it differs. */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const doc = cx.doc;
    const { passed, total } = countPassing(doc, checked.node.id, checked.compiled);
    let changed: boolean;
    try {
      changed = doc.setLayerFilter(checked.node.id, filterIn(input), LAYERS_FILTER_LABEL);
    } catch (e) {
      if (e instanceof Refusal) return failed(error('layer_refused', e.message, 'layer'));
      throw e;
    }
    return { status: 'completed', output: { layer: input.layer, changed, passed, total, revision: String(doc.revision) }, warnings: [] };
  },
};
