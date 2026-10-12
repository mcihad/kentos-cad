import type { LayerCompose } from '../contracts/generated/LayerCompose';
import type { LayerNode as ContractLayerNode } from '../contracts/generated/LayerNode';
import type { LayersCompose } from '../contracts/generated/LayersCompose';
import type { LayersComposed } from '../contracts/generated/LayersComposed';
import type { LayersComposePlan } from '../contracts/generated/LayersComposePlan';
import { Refusal, type CadDocument } from '../model/document';
import { composeOf, maskProblem } from '../model/layerComposeRules';
import type { LayerNode } from '../model/layers';
import { sameJson } from '../model/sameJson';
import { checkRevision, error, failed, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.layers.compose` v1 (docs/adr/0226 §3, §4): a layer's opacity, blending mode and mask written as one undo step
 * “Katman çizimi”; the layer's composition becomes exactly what is given, a layer that already has it is left as it is.
 * The web's handler over `CadDocument`; the desktop's is `crates/native/application/src/layers_compose.rs`. Both pass
 * the shared cases in fixtures/commands/v1/cad.layers.compose.json.
 *
 * The checks, in the contract's order: the opacity, the mask's rule; the expected revision; the layer is the drawing's
 * and a layer; each of the mask's targets is another layer of the drawing.
 */

/** The undo step's name. */
export const LAYERS_COMPOSE_LABEL = 'Katman çizimi';

/** The composition as the layer keeps it (`LayersCompose::composition`): without what says nothing. */
const composition = (input: LayersCompose): LayerCompose | null =>
  composeOf({ ...(input.opacity != null && { opacity: input.opacity }), ...(input.blend != null && { blend: input.blend }), ...(input.mask && { mask: input.mask }) });

function check(doc: CadDocument, input: LayersCompose): Stop | LayerNode {
  const o = input.opacity;
  if (o != null && (!Number.isFinite(o) || o < 0 || o > 1)) return failed(error('invalid_compose', 'Katman çizimi: saydamlık 0 ile 1 arasında olmalı.', 'opacity'));
  const fault = input.mask ? maskProblem(input.mask) : null;
  if (fault) return failed(error('invalid_compose', `Katman çizimi: ${fault}.`, 'mask'));
  const stale = checkRevision(doc, input.expectedRevision);
  if (stale) return stale;
  const id = input.layer;
  const node = doc.layers.get(id);
  if (!node) return failed(error('layer_not_found', `“${id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.`, 'layer'));
  if (node.type !== 'layer') return failed(error('not_a_layer', `“${node.name}” bir katman grubu; katman çizimi yalnız katmanın olur. Grubun bir katmanını verin.`, 'layer'));
  for (const [i, t] of (input.mask?.targets ?? []).entries()) {
    const path = `mask/targets/${i}`;
    if (t === id) return failed(error('mask_target_self', `“${node.name}” kendini maskeleyemez; maskelenecek başka bir katman verin.`, path));
    const target = doc.layers.get(t);
    if (!target) return failed(error('mask_target_not_found', `Maskelenecek “${t}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.`, path));
    if (target.type !== 'layer') return failed(error('mask_target_not_a_layer', `“${target.name}” bir katman grubu; maske katmana uygulanır. Grubun katmanlarını verin.`, path));
  }
  return node;
}

const isStop = (c: Stop | LayerNode): c is Stop => 'status' in c;

export const layersCompose: ProductCommand<LayersCompose, LayersComposed, LayersComposePlan> = {
  id: 'cad.layers.compose',
  version: 1,

  validate(cx, input) {
    const checked = check(cx.doc, input);
    return validated(isStop(checked) ? checked : []);
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const after = composition(input);
    const { compose: before, ...rest } = structuredClone(checked);
    const node = { ...rest, ...(after && { compose: after }) };
    return {
      status: 'completed',
      output: { node: node as unknown as ContractLayerNode, changed: !sameJson(before ? composeOf(before) : null, after), revision: String(cx.doc.revision) },
      warnings: [],
    };
  },

  /** Writes the composition as one undo step “Katman çizimi” when it differs. */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const doc = cx.doc;
    let changed: boolean;
    try {
      changed = doc.setLayerCompose(checked.id, composition(input), LAYERS_COMPOSE_LABEL);
    } catch (e) {
      if (e instanceof Refusal) return failed(error('layer_refused', e.message, 'layer'));
      throw e;
    }
    return { status: 'completed', output: { layer: input.layer, changed, revision: String(doc.revision) }, warnings: [] };
  },
};
