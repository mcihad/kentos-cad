import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { NetworkDef } from '../contracts/generated/NetworkDef';
import type { NetworkDefine } from '../contracts/generated/NetworkDefine';
import type { NetworkDefined } from '../contracts/generated/NetworkDefined';
import type { CadDocument } from '../model/document';
import { networkLayers, networkProblem, networksProblem } from '../model/networkRules';
import { checkRevision, error, failed, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.network.define` v1 (docs/adr/0209 §11): a network's definition written into the project's settings (`set`: in
 * place of the one of its id, else last) or taken away (`remove`). A network is a project setting, as the topology rules
 * are: the drawing becomes dirty and no undo step is written. The web's handler over `CadDocument`; the desktop's is
 * `crates/native/application/src/network_define.rs`. Both pass the shared cases in
 * fixtures/commands/v1/cad.network.define.json.
 *
 * The checks, in the contract's order: what the operation needs (`network`, `id`); the network's and the list's rules;
 * the expected revision; the network removed is the project's. A layer the drawing does not have is a warning.
 */

interface Checked {
  networks: NetworkDef[];
  warnings: CommandWarning[];
}

function check(doc: CadDocument, input: NetworkDefine): Stop | Checked {
  const own = doc.settings.networks.value;
  if (input.operation === 'set' && !input.network) return failed(error('no_network', 'Yazılacak ağ (network) verilmedi; ağın tanımını verin.', 'network'));
  if (input.operation === 'remove' && (input.id === undefined || input.id === ''))
    return failed(error('no_id', 'Silinecek ağın kimliğini (id) verin.', 'id'));
  let networks: NetworkDef[];
  const warnings: CommandWarning[] = [];
  if (input.operation === 'set') {
    const n = input.network!;
    const at = own.findIndex((o) => o.id === n.id);
    networks = structuredClone([...own]);
    if (at < 0) networks.push(structuredClone(n));
    else networks[at] = structuredClone(n);
    const wrong = networkProblem(n) ?? networksProblem(networks);
    if (wrong) return failed(error('invalid_network', wrong, 'network'));
    for (const layer of networkLayers(n))
      if (!doc.layers.get(layer))
        warnings.push({ code: 'unknown_layer', message: `“${layer}” kimlikli katman çizimde yok; ağ kurulurken bu katman atlanır.`, path: 'network' });
  } else networks = own.filter((o) => o.id !== input.id).map((o) => structuredClone(o));
  const stale = checkRevision(doc, input.expectedRevision);
  if (stale) return stale;
  if (input.operation === 'remove' && networks.length === own.length)
    return failed(error('unknown_network', `“${input.id}” kimlikli ağ projede yok. Var olan bir ağın kimliğini verin.`, 'id'));
  return { networks, warnings };
}

const isStop = (c: Stop | Checked): c is Stop => 'status' in c;

export const networkDefine: ProductCommand<NetworkDefine, NetworkDefined, NetworkDefined> = {
  id: 'cad.network.define',
  version: 1,

  validate(cx, input) {
    const checked = check(cx.doc, input);
    return validated(isStop(checked) ? checked : checked.warnings);
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    return { status: 'completed', output: { networks: checked.networks, revision: String(cx.doc.revision) }, warnings: checked.warnings };
  },

  /** Writes the project's networks; a setting, not an undo step. */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    cx.doc.settings.assign({ networks: checked.networks });
    return { status: 'completed', output: { networks: checked.networks, revision: String(cx.doc.revision) }, warnings: checked.warnings };
  },
};
