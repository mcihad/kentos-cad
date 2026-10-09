import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { LayerNode as ContractLayerNode } from '../contracts/generated/LayerNode';
import type { LayerServiceOperation } from '../contracts/generated/LayerServiceOperation';
import type { LayersService } from '../contracts/generated/LayersService';
import type { LayersServiced } from '../contracts/generated/LayersServiced';
import type { LayersServicePlan } from '../contracts/generated/LayersServicePlan';
import type { ServiceConnection } from '../contracts/generated/ServiceConnection';
import { Refusal, type CadDocument, type ServedBy } from '../model/document';
import { fieldsProblem } from '../model/layerFields';
import type { LayerInit, LayerNode } from '../model/layers';
import { connectionsProblem, feedProblem, serviceProblem } from '../model/serviceRules';
import { checkRevision, error, failed, isBlank, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.layers.service` v1 (docs/adr/0208 §15): one change of the layer tree's service layers as one undo step named
 * after the operation: a layer drawn from a map service added (“Harita servisi ekle”), a layer for a service's
 * objects added with its fields (“Veri katmanı ekle”), one changed (“Servis katmanını değiştir”: its name, service or
 * source) or a service layer removed (“Servis katmanını sil”). The project's connections the input brings are added
 * or replace those of the same id first; project settings are not undo steps, so they stay after an undo.
 * The web's handler over `CadDocument`; the desktop's is `crates/native/application/src/layers_service.rs`. Both pass
 * the shared cases in fixtures/commands/v1/cad.layers.service.json.
 *
 * The checks, in the contract's order: the layer an update or a removal needs; the name; what the operation needs (a
 * service, a source, not both); the service's, the source's, the fields' and the connections' rules; the expected
 * revision; the layer is the drawing's, a layer, one of the kind the operation changes; a connection named is the
 * project's (after the input's).
 */

/** The undo step's name (docs/adr/0208 §15). */
export const LAYER_SERVICE_LABEL: Record<LayerServiceOperation, string> = {
  add: 'Harita servisi ekle',
  addFeed: 'Veri katmanı ekle',
  update: 'Servis katmanını değiştir',
  remove: 'Servis katmanını sil',
};

interface Checked {
  /** The project's connections after the input's. */
  connections: ServiceConnection[];
  /** `update`, `remove`: the layer. */
  node: LayerNode | null;
}

/** The project's connections with the input's added, or replacing those of the same id in their place. */
function merged(own: readonly ServiceConnection[], given: readonly ServiceConnection[] | undefined): ServiceConnection[] {
  const out = structuredClone([...own]);
  for (const c of given ?? []) {
    const at = out.findIndex((o) => o.id === c.id);
    if (at < 0) out.push(structuredClone(c));
    else out[at] = structuredClone(c);
  }
  return out;
}

function check(doc: CadDocument, input: LayersService): Stop | Checked {
  const op = input.operation;
  const changes = op === 'update' || op === 'remove';
  if (changes && (input.layer === undefined || input.layer === ''))
    return failed(error('no_layer', 'Değiştirilecek ya da silinecek katmanın kimliğini (layer) verin.', 'layer'));
  if ((op === 'add' || op === 'addFeed' || input.name !== undefined) && isBlank(input.name ?? ''))
    return failed(error('empty_name', 'Katmanın adı boş olamaz; bir ad verin.', 'name'));
  if (op === 'add' && !input.service) return failed(error('no_service', 'Eklenecek servis (service) verilmedi; servisin türünü ve adresini verin.', 'service'));
  if (op === 'addFeed' && !input.feed)
    return failed(error('no_feed', 'Veri katmanının kaynağı (feed) verilmedi; servisin türünü, adresini ve tür adını verin.', 'feed'));
  if (input.service && input.feed)
    return failed(error('service_and_feed', 'Bir katman ya servisten çizilir ya nesnelerini bir kaynaktan alır; service ile feed birlikte verilmez.', 'feed'));
  const serviceWrong = input.service && serviceProblem(input.service);
  if (serviceWrong) return failed(error('invalid_service', serviceWrong, 'service'));
  const feedWrong = input.feed && feedProblem(input.feed);
  if (feedWrong) return failed(error('invalid_feed', feedWrong, 'feed'));
  const fieldsWrong = input.fields?.length ? fieldsProblem(input.fields) : null;
  if (fieldsWrong) return failed(error('invalid_fields', fieldsWrong, 'fields'));
  const connections = merged(doc.settings.connections.value, input.connections);
  const connectionsWrong = input.connections ? connectionsProblem(connections) : null;
  if (connectionsWrong) return failed(error('invalid_connection', connectionsWrong, 'connections'));
  const stale = checkRevision(doc, input.expectedRevision);
  if (stale) return stale;
  let node: LayerNode | null = null;
  if (changes) {
    const id = input.layer!;
    node = doc.layers.get(id) ?? null;
    if (!node) return failed(error('layer_not_found', `“${id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.`, 'layer'));
    if (node.type !== 'layer') return failed(error('not_a_layer', `“${node.name}” bir katman grubu; servis ve veri kaynağı yalnız katmanın olur.`, 'layer'));
    if ((op === 'remove' || input.service) && !node.service)
      return failed(
        error(
          'not_a_service_layer',
          op === 'remove'
            ? `“${node.name}” bir servis katmanı değil; bu komut yalnız servis katmanını siler. Katmanı Katmanlar panelinden silin.`
            : `“${node.name}” bir servis katmanı değil; servisi yalnız servis katmanının değişir.`,
          'layer',
        ),
      );
    if (input.feed && !node.feed)
      return failed(error('not_a_service_layer', `“${node.name}” katmanının veri kaynağı yok; kaynak yalnız veri katmanının değişir.`, 'layer'));
    if (input.service && doc.byLayer(id).length)
      return failed(error('layer_has_objects', `“${node.name}” katmanında nesne var; servis katmanı nesne tutmaz.`, 'layer'));
  }
  const known = new Set(connections.map((c) => c.id));
  for (const [named, path] of [
    [input.service?.connection, 'service.connection'],
    [input.feed?.connection, 'feed.connection'],
  ] as const)
    if (named !== undefined && !known.has(named))
      return failed(
        error('unknown_connection', `“${named}” bağlantısı projede yok; bağlantıyı connections ile birlikte verin ya da Bağlantılar penceresinden ekleyin.`, path),
      );
  return { connections, node };
}

const isStop = (c: Stop | Checked): c is Stop => 'status' in c;

/** The new layer of `add` and `addFeed`, as the tree would make it. */
function init(input: LayersService): LayerInit {
  return {
    name: input.name!.trim(),
    type: 'layer',
    ...(input.service && { service: input.service }),
    ...(input.feed && { feed: input.feed }),
    ...(input.fields?.length && { fields: input.fields }),
  };
}

export const layersService: ProductCommand<LayersService, LayersServiced, LayersServicePlan> = {
  id: 'cad.layers.service',
  version: 1,

  validate(cx, input) {
    const checked = check(cx.doc, input);
    return validated(isStop(checked) ? checked : []);
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const doc = cx.doc;
    let node: LayerNode | null = null;
    if (input.operation === 'add' || input.operation === 'addFeed') node = doc.layers.preview(init(input));
    else if (input.operation === 'update' && checked.node) {
      const n = checked.node;
      const { service: _service, feed: _feed, children: _children, ...rest } = structuredClone(n);
      node = {
        ...rest,
        name: input.name?.trim() ?? n.name,
        children: [],
        ...((input.service ?? n.service) && { service: structuredClone(input.service ?? n.service) }),
        ...((input.feed ?? n.feed) && { feed: structuredClone(input.feed ?? n.feed) }),
      };
    }
    const warnings: CommandWarning[] = [];
    return {
      status: 'completed',
      output: { ...(node && { node: node as unknown as ContractLayerNode }), connections: checked.connections, revision: String(doc.revision) },
      warnings,
    };
  },

  /** Writes the change as one undo step named after the operation. */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const doc = cx.doc;
    const label = LAYER_SERVICE_LABEL[input.operation];
    if (input.connections) doc.settings.assign({ connections: checked.connections });
    let layer: string;
    try {
      if (input.operation === 'add' || input.operation === 'addFeed') {
        layer = doc.addLayer(init(input), input.parent ?? null, { index: input.index, label }).id;
      } else if (input.operation === 'update') {
        const n = checked.node!;
        const next: ServedBy = {
          ...((input.service ?? n.service) && { service: input.service ?? n.service }),
          ...((input.feed ?? n.feed) && { feed: input.feed ?? n.feed }),
        };
        doc.setLayerService(n.id, next, label, input.name);
        layer = n.id;
      } else {
        layer = checked.node!.id;
        doc.transact(label, () => {
          doc.removeLayer(layer);
        });
      }
    } catch (e) {
      if (e instanceof Refusal) return failed(error('layer_refused', e.message, 'layer'));
      throw e;
    }
    return { status: 'completed', output: { layer, revision: String(doc.revision) }, warnings: [] };
  },
};
