import type { AppContext } from './context';

/**
 * Kapsam denetimi (`view.extentCheck`, docs/adr/0141): selects the visible objects that lie far from
 * the rest of the drawing (brought in with a wrong coordinate system, fallen to zero) and says how
 * many. The store's `extent_outliers` decides which; nothing is moved or deleted, that is the user's
 * to decide. When none is found the selection is left as it was.
 */
export function checkExtent(ctx: Pick<AppContext, 'view' | 'selection' | 'log'>): void {
  const far = ctx.view.extentOutliers();
  if (!far.length) return ctx.log.info('Çizimin kapsamını bozan nesne yok.');
  ctx.selection.set(far);
  ctx.log.warn(`${far.length} nesne çizimin geri kalanından çok uzakta; seçildi.`);
}
