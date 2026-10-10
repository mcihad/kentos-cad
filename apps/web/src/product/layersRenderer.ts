import type { LayerNode as ContractLayerNode } from '../contracts/generated/LayerNode';
import type { LayersRendered } from '../contracts/generated/LayersRendered';
import type { LayersRenderer } from '../contracts/generated/LayersRenderer';
import type { LayersRendererPlan } from '../contracts/generated/LayersRendererPlan';
import type { CadDocument } from '../model/document';
import type { LayerNode, LayerStyle } from '../model/layers';
import type { LayerRenderer } from '../model/style';
import { rendererProblem } from '../wasm/core';
import { checkRevision, error, failed, validated, type Stop } from './checks';
import type { ProductCommand } from './command';
import { sameData } from './layersLabels';

/**
 * `cad.layers.renderer` v1 (docs/adr/0213 §5): a layer's renderer written, or taken away (the layer's simple look), as
 * one undo step “Katman stili”; a layer that already has it is left as it is. The web's handler over `CadDocument`;
 * the desktop's is `crates/native/application/src/layers_renderer.rs`. Both pass the shared cases in
 * fixtures/commands/v1/cad.layers.renderer.json.
 *
 * The checks, in the contract's order: the style core's rules of a renderer (`renderer_problem`, through WASM: the
 * same answer as the desktop's); the expected revision; the layer is the drawing's, a layer, not drawn from a service.
 */

/** The undo step's name. */
export const LAYERS_RENDERER_LABEL = 'Katman stili';

interface Checked {
  readonly node: LayerNode;
  readonly style: LayerStyle;
}

function check(doc: CadDocument, input: LayersRenderer): Stop | Checked {
  const renderer = input.renderer ?? null;
  if (renderer !== null) {
    const why = rendererProblem(renderer);
    if (why) return failed(error('invalid_renderer', `İşleyici: ${why}`, 'renderer'));
  }
  const stale = checkRevision(doc, input.expectedRevision);
  if (stale) return stale;
  const id = input.layer;
  const node = doc.layers.get(id);
  if (!node) return failed(error('layer_not_found', `“${id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.`, 'layer'));
  if (node.type !== 'layer')
    return failed(error('not_a_layer', `“${node.name}” bir katman grubu; işleyici yalnız katmanın olur. Grubun bir katmanını verin.`, 'layer'));
  if (node.service)
    return failed(error('service_layer', `“${node.name}” servisten çizilir ve nesne tutmaz; işleyici nesneleri olan katmanın olur.`, 'layer'));
  const style: LayerStyle = structuredClone(node.style);
  if (renderer === null) delete style.renderer;
  else style.renderer = structuredClone(renderer) as LayerRenderer;
  return { node, style };
}

const isStop = (c: Stop | Checked): c is Stop => 'status' in c;

export const layersRenderer: ProductCommand<LayersRenderer, LayersRendered, LayersRendererPlan> = {
  id: 'cad.layers.renderer',
  version: 1,

  validate(cx, input) {
    const checked = check(cx.doc, input);
    return validated(isStop(checked) ? checked : []);
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const node = { ...structuredClone(checked.node), style: checked.style };
    return {
      status: 'completed',
      output: { node: node as unknown as ContractLayerNode, changed: !sameData(checked.node.style, checked.style), revision: String(cx.doc.revision) },
      warnings: [],
    };
  },

  /** Writes the renderer as one undo step “Katman stili” when it differs. */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const doc = cx.doc;
    const changed = !sameData(checked.node.style, checked.style);
    if (changed) doc.setLayerStyle(checked.node.id, { renderer: checked.style.renderer }, LAYERS_RENDERER_LABEL);
    return { status: 'completed', output: { layer: input.layer, changed, revision: String(doc.revision) }, warnings: [] };
  },
};
