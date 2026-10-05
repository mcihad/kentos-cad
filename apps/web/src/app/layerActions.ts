import type { AppContext } from './context';
import { nodesText } from '../tools/layerTools';

/**
 * The layer commands that are no tool (docs/adr/0177; the desktop's `layer_actions.rs`). Yalıtımı kaldır (§1) shows again
 * the layers and groups Katmanı yalıt hid since the last time (`ToolManager.isolatedLayers`) and still hidden; those
 * changed by hand meanwhile stay as they are. A tree change, as the eye: no undo step.
 */
export function unisolateLayers(ctx: AppContext): void {
  const { doc, log, tools } = ctx;
  const layers = doc.layers;
  const remembered = [...tools.isolatedLayers];
  tools.isolatedLayers.length = 0;
  if (!remembered.length) return void log.info('Yalıtılmış katman yok: Katmanı yalıt bu çizimde katman gizlemedi.');
  const back = remembered.filter((id) => layers.get(id)?.visible === false);
  if (!back.length) return void log.info('Katmanı yalıt’ın gizlediği katmanlar zaten görünüyor.');
  for (const id of back) layers.setVisible(id, true);
  const groups = back.filter((id) => layers.get(id)?.type === 'group').length;
  log.success(`Yalıtım kaldırıldı: ${nodesText(back.length - groups, groups)} yeniden gösterildi.`);
}
