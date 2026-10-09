import type { LayerNode as ContractLayerNode } from '../contracts/generated/LayerNode';
import type { LayersTime } from '../contracts/generated/LayersTime';
import type { LayersTimed } from '../contracts/generated/LayersTimed';
import type { LayersTimePlan } from '../contracts/generated/LayersTimePlan';
import { Refusal, type CadDocument } from '../model/document';
import { temporalOf, type LayerNode } from '../model/layers';
import { sameJson } from '../model/sameJson';
import { layerTimeProblem } from '../model/temporalRules';
import { checkRevision, error, failed, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.layers.time` v1 (docs/adr/0210 §11): a layer's time setting written, or taken away, as one undo step “Zaman
 * ayarları”; a layer that already has it as given is left as it is. The web's handler over `CadDocument`; the
 * desktop's is `crates/native/application/src/layers_time.rs`. Both pass the shared cases in
 * fixtures/commands/v1/cad.layers.time.json.
 *
 * The checks, in the contract's order: the setting's rules; the expected revision; the layer is the drawing's, a
 * layer, not drawn from a service.
 */

/** The undo step's name. */
export const LAYERS_TIME_LABEL = 'Zaman ayarları';

function check(doc: CadDocument, input: LayersTime): Stop | LayerNode {
  const problem = input.time && layerTimeProblem(input.time);
  if (problem) return failed(error('invalid_time', `Katmanın zamanı: ${problem}.`, 'time'));
  const stale = checkRevision(doc, input.expectedRevision);
  if (stale) return stale;
  const id = input.layer;
  const node = doc.layers.get(id);
  if (!node) return failed(error('layer_not_found', `“${id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.`, 'layer'));
  if (node.type !== 'layer') return failed(error('not_a_layer', `“${node.name}” bir katman grubu; zaman yalnız katmanın olur. Grubun bir katmanını verin.`, 'layer'));
  if (node.service && input.time)
    return failed(error('service_layer', `“${node.name}” servisten çizilir ve nesne tutmaz; zaman nesneleri olan katmanın olur.`, 'layer'));
  return node;
}

const isStop = (c: Stop | LayerNode): c is Stop => 'status' in c;

/** The input's setting as a layer keeps it (`cumulative` only when true); none when it is taken away. */
const timeOf = (input: LayersTime) => (input.time ? temporalOf({ time: input.time }, 'layer').time : undefined);

export const layersTime: ProductCommand<LayersTime, LayersTimed, LayersTimePlan> = {
  id: 'cad.layers.time',
  version: 1,

  validate(cx, input) {
    const checked = check(cx.doc, input);
    return validated(isStop(checked) ? checked : []);
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const time = timeOf(input);
    const { time: _time, ...rest } = structuredClone(checked);
    const node = { ...rest, ...(time && { time }) };
    return {
      status: 'completed',
      output: { node: node as unknown as ContractLayerNode, changed: !sameJson(checked.time ?? null, time ?? null), revision: String(cx.doc.revision) },
      warnings: [],
    };
  },

  /** Writes the setting as one undo step “Zaman ayarları” when it differs. */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const doc = cx.doc;
    let changed: boolean;
    try {
      changed = doc.setLayerTime(checked.id, timeOf(input), LAYERS_TIME_LABEL);
    } catch (e) {
      if (e instanceof Refusal) return failed(error('layer_refused', e.message, 'layer'));
      throw e;
    }
    return { status: 'completed', output: { layer: input.layer, changed, revision: String(doc.revision) }, warnings: [] };
  },
};
