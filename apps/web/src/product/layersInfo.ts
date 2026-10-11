import type { InfoCard } from '../contracts/generated/InfoCard';
import type { LayerNode as ContractLayerNode } from '../contracts/generated/LayerNode';
import type { LayersInfo } from '../contracts/generated/LayersInfo';
import type { LayersInformed } from '../contracts/generated/LayersInformed';
import type { LayersInfoPlan } from '../contracts/generated/LayersInfoPlan';
import { Refusal, type CadDocument } from '../model/document';
import { compileTemplate } from '../model/infoTemplates';
import { actionsProblem, cardProblem } from '../model/layerInfoRules';
import { infoOf, type LayerInfo, type LayerNode } from '../model/layers';
import { sameJson } from '../model/sameJson';
import { checkRevision, error, failed, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.layers.info` v1 (docs/adr/0225 §4, §5): a layer's actions and information card written as one undo step “Bilgi
 * kartı”; a layer that already has them as given is left as it is. A card with nothing in it is no card. The web's
 * handler over `CadDocument`; the desktop's is `crates/native/application/src/layers_info.rs`. Both pass the shared
 * cases in fixtures/commands/v1/cad.layers.info.json.
 *
 * The checks, in the contract's order: the actions' rules, the card's; the templates split and their expressions
 * compile (`$sıra` and `$ölçek` refused), the actions' targets in order, then the card's title and text; the expected
 * revision; the layer is the drawing's, a layer, not drawn from a service.
 */

/** The undo step's name. */
export const LAYERS_INFO_LABEL = 'Bilgi kartı';

/** Whether a card has nothing in it (the default card). */
const emptyCard = (c: InfoCard): boolean => c.title == null && !c.fields?.length && c.text == null && c.attachments !== false && c.actions !== false;

/** The input as the layer will keep it: a card with nothing in it is none. */
const infoIn = (input: LayersInfo): LayerInfo => ({ ...(input.actions?.length && { actions: input.actions }), ...(input.card && !emptyCard(input.card) && { card: input.card }) });

function check(doc: CadDocument, input: LayersInfo): Stop | LayerNode {
  const problem = actionsProblem(input.actions ?? []);
  if (problem) return failed(error('invalid_actions', `Katmanın eylemleri: ${problem}.`, 'actions'));
  const cardFault = input.card ? cardProblem(input.card) : null;
  if (cardFault) return failed(error('invalid_card', `Katmanın bilgi kartı: ${cardFault}.`, 'card'));
  for (const [i, a] of (input.actions ?? []).entries()) {
    const t = compileTemplate(a.target);
    if (!t.ok) return failed(error('invalid_template', `“${a.name}” eyleminin hedefi: ${t.error}`, `actions/${i}/target`));
  }
  for (const [text, what, path] of [
    [input.card?.title, 'Kartın başlığı', 'card/title'],
    [input.card?.text, 'Kartın metni', 'card/text'],
  ] as const) {
    if (text == null) continue;
    const t = compileTemplate(text);
    if (!t.ok) return failed(error('invalid_template', `${what}: ${t.error}`, path));
  }
  const stale = checkRevision(doc, input.expectedRevision);
  if (stale) return stale;
  const id = input.layer;
  const node = doc.layers.get(id);
  if (!node) return failed(error('layer_not_found', `“${id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.`, 'layer'));
  if (node.type !== 'layer')
    return failed(error('not_a_layer', `“${node.name}” bir katman grubu; eylemler ve bilgi kartı yalnız katmanın olur. Grubun bir katmanını verin.`, 'layer'));
  const info = infoIn(input);
  if (node.service && (info.actions || info.card))
    return failed(error('service_layer', `“${node.name}” servisten çizilir ve nesne tutmaz; eylemler ve bilgi kartı nesneleri olan katmanın olur.`, 'layer'));
  return node;
}

const isStop = (c: Stop | LayerNode): c is Stop => 'status' in c;

export const layersInfo: ProductCommand<LayersInfo, LayersInformed, LayersInfoPlan> = {
  id: 'cad.layers.info',
  version: 1,

  validate(cx, input) {
    const checked = check(cx.doc, input);
    return validated(isStop(checked) ? checked : []);
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const after = infoOf(infoIn(input), 'layer');
    const { actions: _a, card: _c, ...rest } = structuredClone(checked);
    const node = { ...rest, ...after };
    return {
      status: 'completed',
      output: { node: node as unknown as ContractLayerNode, changed: !sameJson(infoOf(checked, checked.type), after), revision: String(cx.doc.revision) },
      warnings: [],
    };
  },

  /** Writes the actions and the card as one undo step “Bilgi kartı” when they differ. */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const doc = cx.doc;
    let changed: boolean;
    try {
      changed = doc.setLayerInfo(checked.id, infoIn(input), LAYERS_INFO_LABEL);
    } catch (e) {
      if (e instanceof Refusal) return failed(error('layer_refused', e.message, 'layer'));
      throw e;
    }
    return { status: 'completed', output: { layer: input.layer, changed, revision: String(doc.revision) }, warnings: [] };
  },
};
