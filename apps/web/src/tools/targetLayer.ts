import type { AppContext } from '../app/context';

/**
 * The layer new objects go to: `preferred` or the active one. Null, with
 * a message saying how to fix it, when that layer is locked; a hidden
 * layer is allowed with a warning.
 */
export function writableLayer(ctx: AppContext, preferred?: string): string | null {
  const layers = ctx.doc.layers;
  const id = preferred ?? layers.active.value;
  const node = layers.get(id);
  if (!node) return null;
  if (layers.isLocked(id)) {
    ctx.log.warn(`“${node.name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.`);
    return null;
  }
  if (!layers.isVisible(id)) ctx.log.warn(`“${node.name}” katmanı gizli; çizilen nesne görünmeyecek.`);
  return id;
}
