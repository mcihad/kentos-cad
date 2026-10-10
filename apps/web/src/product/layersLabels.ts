import type { LayerNode as ContractLayerNode } from '../contracts/generated/LayerNode';
import type { LayersLabelled } from '../contracts/generated/LayersLabelled';
import type { LayersLabels } from '../contracts/generated/LayersLabels';
import type { LayersLabelsPlan } from '../contracts/generated/LayersLabelsPlan';
import type { CadDocument } from '../model/document';
import { labelStyleProblem, layerLabelsProblem } from '../model/labelRules';
import { labelExpression } from '../model/labelTexts';
import type { LayerNode, LayerStyle } from '../model/layers';
import { checkRevision, error, failed, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.layers.labels` v1 (docs/adr/0212 §5): a layer's labelling (its single label's style and how it is labelled)
 * written as one undo step “Etiketler”; a layer that already has it as given is left as it is. The web's handler over
 * `CadDocument`; the desktop's is `crates/native/application/src/layers_labels.rs`. Both pass the shared cases in
 * fixtures/commands/v1/cad.layers.labels.json.
 *
 * The checks, in the contract's order: the style's and the labelling's rules; their texts' and conditions'
 * expressions compile (`$sıra` and `$ölçek` refused); the expected revision; the layer is the drawing's, a layer, not
 * drawn from a service.
 */

/** The undo step's name. */
export const LAYERS_LABELS_LABEL = 'Etiketler';

interface Checked {
  readonly node: LayerNode;
  readonly style: LayerStyle;
}

/** Equality of plain data as Rust's derived `PartialEq` has it: keys in any order, an undefined property absent. */
export function sameData(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) return false;
  if (Array.isArray(a) || Array.isArray(b)) {
    if (!Array.isArray(a) || !Array.isArray(b) || a.length !== b.length) return false;
    return a.every((x, i) => sameData(x, b[i]));
  }
  const x = a as Record<string, unknown>;
  const y = b as Record<string, unknown>;
  const keys = (o: Record<string, unknown>) => Object.keys(o).filter((k) => o[k] !== undefined);
  const kx = keys(x);
  return kx.length === keys(y).length && kx.every((k) => y[k] !== undefined && sameData(x[k], y[k]));
}

function compiles(source: string | undefined, what: string, path: string): Stop | null {
  if (source === undefined) return null;
  const c = labelExpression(source);
  return c.error ? failed(error('invalid_expression', `${what}: ${c.error}`, path)) : null;
}

function check(doc: CadDocument, input: LayersLabels): Stop | Checked {
  if (input.label) {
    const p = labelStyleProblem(input.label);
    if (p) return failed(error('invalid_labels', `Etiketin stili: ${p}.`, 'label'));
  }
  if (input.labels) {
    const p = layerLabelsProblem(input.labels);
    if (p) return failed(error('invalid_labels', `Etiketleme: ${p}.`, 'labels'));
  }
  if (input.label) {
    const stop = compiles(input.label.text, 'Etiketin metni', 'label/text');
    if (stop) return stop;
  }
  for (const [i, c] of (input.labels?.classes ?? []).entries()) {
    const stop =
      compiles(c.when, `“${c.name}” sınıfının koşulu`, `labels/classes/${i}/when`) ??
      compiles(c.style.text, `“${c.name}” sınıfının metni`, `labels/classes/${i}/style/text`);
    if (stop) return stop;
  }
  const stale = checkRevision(doc, input.expectedRevision);
  if (stale) return stale;
  const id = input.layer;
  const node = doc.layers.get(id);
  if (!node) return failed(error('layer_not_found', `“${id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.`, 'layer'));
  if (node.type !== 'layer')
    return failed(error('not_a_layer', `“${node.name}” bir katman grubu; etiketleme yalnız katmanın olur. Grubun bir katmanını verin.`, 'layer'));
  if (node.service && (input.label || input.labels))
    return failed(error('service_layer', `“${node.name}” servisten çizilir ve nesne tutmaz; etiketleme nesneleri olan katmanın olur.`, 'layer'));
  const style: LayerStyle = structuredClone(node.style);
  if (input.label !== undefined) {
    if (input.label === null) delete style.label;
    else style.label = structuredClone(input.label);
  }
  if (input.labels !== undefined) {
    if (input.labels === null) delete style.labels;
    else style.labels = structuredClone(input.labels);
  }
  return { node, style };
}

const isStop = (c: Stop | Checked): c is Stop => 'status' in c;

export const layersLabels: ProductCommand<LayersLabels, LayersLabelled, LayersLabelsPlan> = {
  id: 'cad.layers.labels',
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

  /** Writes the labelling as one undo step “Etiketler” when it differs. */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const doc = cx.doc;
    const changed = !sameData(checked.node.style, checked.style);
    if (changed) doc.setLayerStyle(checked.node.id, { label: checked.style.label, labels: checked.style.labels }, LAYERS_LABELS_LABEL);
    return { status: 'completed', output: { layer: input.layer, changed, revision: String(doc.revision) }, warnings: [] };
  },
};
