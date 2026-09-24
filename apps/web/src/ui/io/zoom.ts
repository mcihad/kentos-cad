import type { AppContext } from '../../app/context';

/** At least this much around what was imported (one point alone is shown in context). */
const MIN_SPAN = 20;

/** Shows the objects an import added: their extent, never smaller than a few metres. */
export function zoomToImported(ctx: AppContext, ids: readonly number[]): void {
  const b = ctx.doc.bounds(ids);
  if (!b) return;
  const grow = (lo: number, hi: number) => {
    const pad = Math.max(0, (MIN_SPAN - (hi - lo)) / 2);
    return [lo - pad, hi + pad];
  };
  const [minX, maxX] = grow(b.minX, b.maxX);
  const [minY, maxY] = grow(b.minY, b.maxY);
  ctx.view.camera.fit({ minX, minY, maxX, maxY });
}
