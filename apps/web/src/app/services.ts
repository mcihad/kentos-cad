import type { LayersService } from '../contracts/generated/LayersService';
import type { LayerNode } from '../model/layers';
import { layerOf, type Preset } from '../model/servicePresets';
import { layersService } from '../product/layersService';
import type { AppContext } from './context';

/**
 * Hazır altlıklar (docs/adr/0208 §1, §14): a ready basemap goes to the bottom of the layer tree in place of the ready
 * basemap there (as ArcGIS's Basemap gallery does), else below everything, by `cad.layers.service` as one undo step;
 * Altlığı kaldır takes the bottom basemap away. A basemap that needs a key this device does not have opens Bağlantılar
 * on it. The desktop's are `apps/desktop/src/services/basemaps.rs`.
 */

/** The ready basemap at the bottom of the tree: the last top-level node, when a preset draws it. */
export function bottomBasemap(ctx: AppContext): LayerNode | null {
  const roots = ctx.doc.layers.tree;
  const last = roots[roots.length - 1];
  return last && last.type === 'layer' && last.service?.preset !== undefined ? last : null;
}

function run(ctx: AppContext, input: LayersService): boolean {
  const result = layersService.execute({ doc: ctx.doc }, input);
  if (result.status !== 'completed') {
    if ('error' in result) ctx.log.warn(result.error.message);
    return false;
  }
  for (const w of result.warnings) ctx.log.warn(w.message);
  return true;
}

/** Shows `p` as the basemap: the bottom one changes to it, or it goes below everything. */
export function addBasemap(ctx: AppContext, p: Preset): void {
  const service = layerOf(p);
  const connections = p.connection ? [p.connection] : undefined;
  const bottom = bottomBasemap(ctx);
  const done = bottom
    ? run(ctx, { operation: 'update', layer: bottom.id, name: p.name, service, ...(connections && { connections }) })
    : run(ctx, { operation: 'add', name: p.name, service, ...(connections && { connections }) });
  if (!done) return;
  ctx.log.success(bottom ? `Altlık “${p.name}” oldu.` : `“${p.name}” altlık olarak eklendi.`);
  const c = p.connection;
  if (c && ctx.secrets.get(c.origin, c.id) === null) {
    ctx.log.warn(`“${p.name}” bir anahtar ister: Harita › Altlık › Bağlantılar'da “${c.name}” bağlantısının anahtarını girin.`);
    void import('../ui/services/ConnectionsDialog').then((m) => m.openConnections(ctx, { focus: c.id }));
  }
}

/** Altlığı kaldır: the bottom basemap goes, in one undo step. */
export function removeBasemap(ctx: AppContext): void {
  const bottom = bottomBasemap(ctx);
  if (!bottom) {
    ctx.log.info('Kaldırılacak hazır altlık yok.');
    return;
  }
  if (run(ctx, { operation: 'remove', layer: bottom.id })) ctx.log.success(`“${bottom.name}” altlığı kaldırıldı.`);
}
