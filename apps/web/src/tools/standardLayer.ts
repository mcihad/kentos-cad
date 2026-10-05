import type { AppContext } from '../app/context';
import type { CommandResult } from '../contracts/generated/CommandResult';
import { Refusal } from '../model/document';
import type { LayerInit } from '../model/layers';
import { cadLayers, standardLayers } from '../model/standardLayers';
import { error, failed } from '../product/checks';

/**
 * The standard layers some tools write to, whatever layer is active: Parsel
 * oluştur's `parsel`, Kot noktası's `kot` (docs/adr/0067). A drawing without
 * one (an imported plan, an older file) gets it as a new project has it
 * (`standardLayers`: same id, name and style), at the top of the tree and not
 * active, in the undo step of the object written to it; nothing stays when
 * that object is refused. The desktop does the same
 * (`crates/native/interaction/src/standard_layer.rs`).
 */

/** Carries a refused write out of the transaction, which then takes the layer back. */
class Refused extends Error {}

/**
 * The layer of a new project's tree by its id, as a fresh copy; the bare id when the tree lacks it. A CAD project
 * looks in its own tree first (its `yazi` is “Yazı”, docs/adr/0175 §3), then in the CBS one (`parsel`, `kot`).
 */
function standardLayer(id: string, plotScale: number, cad: boolean): LayerInit {
  const find = (nodes: readonly LayerInit[]): LayerInit | undefined => {
    for (const n of nodes) {
      if (n.id === id && !n.children) return n;
      const inside = n.children && find(n.children);
      if (inside) return inside;
    }
    return undefined;
  };
  return (cad ? find(cadLayers()) : undefined) ?? find(standardLayers(plotScale)) ?? { id, name: id };
}

/** The name layer `layerId` has, or the one it is opened with when the drawing lacks it. */
export function standardLayerName(ctx: AppContext, layerId: string): string {
  const { doc } = ctx;
  return doc.layers.get(layerId)?.name ?? standardLayer(layerId, doc.settings.plotScale.value, doc.settings.workspace.value === 'cad').name;
}

/**
 * Runs `write` (a product command writing to layer `layerId`), opening that
 * layer first when the drawing has none: layer and object are one undo step,
 * named `step` (the command's own name for it; “Ekle” by default). Once the
 * object is written, an info message says the layer was opened, `what` being
 * what it was opened for (“parsel”, “kot noktası”); the tool's own messages
 * follow it. A locked layer is not this function's: it is there, and `write`
 * meets it as ever. The command's answer.
 */
export function writeOnStandardLayer<T>(ctx: AppContext, layerId: string, what: string, write: () => CommandResult<T>, step = 'Ekle'): CommandResult<T> {
  const { doc } = ctx;
  if (doc.layers.get(layerId)) return write();
  let result: CommandResult<T> | undefined;
  try {
    doc.transact(step, () => {
      doc.addLayer(standardLayer(layerId, doc.settings.plotScale.value, doc.settings.workspace.value === 'cad'), null);
      result = write();
      if (result.status !== 'completed') throw new Refused();
    });
  } catch (e) {
    if (e instanceof Refused && result) return result;
    if (e instanceof Refusal) return failed(error('layer_not_opened', e.message, 'layerId'));
    throw e;
  }
  const name = doc.layers.get(layerId)?.name ?? layerId;
  ctx.log.info(`“${name}” katmanı çizimde yoktu; ${what} için açıldı.`);
  return result!;
}
