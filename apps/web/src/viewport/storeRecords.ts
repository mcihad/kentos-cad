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
export const LABEL = { dimension: 0, text: 1, center: 2, corner: 3, beside: 4, along: 5, pieceText: 6, pieceDimension: 7 } as const;
/**
 * Numbers per label record: `id, what, x, y, a, b, c, d, e`. A text's x, y are where its baseline starts (its
 * point moved by its alignment), b its width factor, c its mask's width (0 none); a block's text piece's d and e
 * (docs/adr/0145).
 */
export const LABEL_STRIDE = 9;
/** A dimension record's prefix code. */
export const DIMENSION_PREFIX = ['', 'R ', 'Ø '] as const;

/** The grips of one object: points, the segment of each mid grip (−1 for other grips), a path's vertex count. */
export interface GripSet {
  id: number;
  points: Vec2[];
  segments: number[];
  /** A path's vertex count; a multi-part area's, the first part's (docs/adr/0143). */
  vertices: number;
  /**
   * The vertex grips of the ring a mid grip belongs to, by grip index, for the mid grips of the parts of a
   * multi-part area past its first: the store counts a mid grip's segment within its own part, so the segment's
   * ends are those grips' `from + segment` and `from + (segment + 1) % count`. Absent when every mid grip is of the
   * first ring, whose vertex grips are the first `vertices`.
   */
  rings?: ({ from: number; count: number } | undefined)[];
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
    partRings(set);
    out.push(set);
  }
  return out;
}

/**
 * Finds the rings of a multi-part area's parts past its first (docs/adr/0143). A part lists its grips as an area
 * does: its vertices, then a mid grip for each of them, whose segments run 0, 1, 2 …, then its holes' vertices; the
 * first part's mid grips follow the first `vertices` grips. A run of mid grips anywhere else is another part's, and
 * so many mid grips are so many vertices, just before the run. One pass over the grips, whatever their number.
 */
function partRings(set: GripSet): void {
  const n = set.points.length;
  for (let i = 0; i < n; i++) {
    if (set.segments[i] !== 0) continue;
    // The run of mid grips from here on: `k` of them, their segments 0 to k − 1.
    let k = 1;
    while (i + k < n && set.segments[i + k] === k) k++;
    if (i !== set.vertices) {
      set.rings ??= [];
      for (let j = 0; j < k; j++) set.rings[i + j] = { from: i - k, count: k };
    }
    i += k - 1;
  }
}
