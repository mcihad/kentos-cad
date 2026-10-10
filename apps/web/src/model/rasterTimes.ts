// The time slider's range with the rasters following it (docs/adr/0243 §7), twin of the desktop's
// `temporal::with_rasters`: a shown raster whose dataset follows the slider adds its time steps' first and last.

import type { CadDocument } from './document';

/** The temporal layers' count and extent with the following rasters' time steps. */
export function withRasterTimes(
  summary: { count: number; extent: [number, number] | null },
  doc: Pick<CadDocument, 'all' | 'layers'>,
): { count: number; extent: [number, number] | null } {
  let { count, extent } = summary;
  for (const e of doc.all()) {
    if (e.kind !== 'raster' || !e.dataset?.followTime || !doc.layers.isVisible(e.layerId)) continue;
    const t = (e.dataset.dims ?? []).find((d) => d.time);
    const v = t?.values;
    if (!v?.length) continue;
    const [a, b] = [v[0], v[v.length - 1]];
    count += 1;
    extent = extent ? [Math.min(extent[0], a), Math.max(extent[1], b)] : [a, b];
  }
  return { count, extent };
}
