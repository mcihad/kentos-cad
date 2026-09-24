import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import type { LabelStyle } from '../model/layers';

/**
 * What the geometry store hands the overlay (docs/adr/0008, S1): label
 * records and grips, as flat numbers (crates/shared/geometry-core/src/store/
 * labels.rs). The store decides which labels a frame draws and where; the
 * overlay reads their strings and styles from the objects.
 */

/** Labels a layer without a label style gets, by kind. */
export const DEFAULT_LABELS: Partial<Record<Entity['kind'], LabelStyle>> = {
  polygon: { placement: 'center', size: 10, grow: 1, maxSize: 14, minFeaturePx: 26 },
  circle: { placement: 'center', size: 10, minFeaturePx: 26 },
  point: { placement: 'beside', size: 10.5, minScale: 2 },
  polyline: { placement: 'along', size: 10, minScale: 1.6 },
  line: { placement: 'along', size: 10, minScale: 1.6 },
};

/** What decides whether and where a label is drawn, as the store reads it. */
export function labelRule(st: LabelStyle): { placement: LabelStyle['placement']; minScale?: number; maxScale?: number; minFeaturePx?: number } {
  return { placement: st.placement, minScale: st.minScale, maxScale: st.maxScale, minFeaturePx: st.minFeaturePx };
}

/** A label record's second number. */
export const LABEL = { dimension: 0, text: 1, center: 2, corner: 3, beside: 4, along: 5 } as const;
/** Numbers per label record: `id, what, x, y, a, b, c, d`. */
export const LABEL_STRIDE = 8;
/** A dimension record's prefix code. */
export const DIMENSION_PREFIX = ['', 'R ', 'Ø '] as const;

/** The grips of one object: points, the segment of each mid grip (−1 for other grips), a path's vertex count. */
export interface GripSet {
  id: number;
  points: Vec2[];
  segments: number[];
  vertices: number;
}

/** Grips as the store lists them: `id, count, vertices`, then `x, y, segment` per grip. */
export function readGrips(f: Float64Array): GripSet[] {
  const out: GripSet[] = [];
  for (let i = 0; i + 2 < f.length; ) {
    const set: GripSet = { id: f[i], points: [], segments: [], vertices: f[i + 2] };
    const n = f[i + 1];
    i += 3;
    for (let k = 0; k < n; k++, i += 3) {
      set.points.push({ x: f[i], y: f[i + 1] });
      set.segments.push(f[i + 2]);
    }
    out.push(set);
  }
  return out;
}
