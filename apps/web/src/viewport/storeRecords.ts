import type { Vec2 } from '../model/geometry';
import type { LabelStyle } from '../model/layers';

/**
 * What the geometry store hands the overlay (docs/adr/0008, S1): label
 * records and grips, as flat numbers (crates/shared/geometry-core/src/store/
 * labels.rs). The store decides which labels a frame draws and where; the
 * overlay reads their strings and styles from the objects.
 */

/** Labels a layer without a label style gets, by kind (the model's, docs/adr/0175 §4). */
export { DEFAULT_LABELS } from '../model/labelDefaults';

/** What decides whether and where a label is drawn, as the store reads it. */
export function labelRule(st: LabelStyle): { placement: LabelStyle['placement']; minScale?: number; maxScale?: number; minFeaturePx?: number } {
  return { placement: st.placement, minScale: st.minScale, maxScale: st.maxScale, minFeaturePx: st.minFeaturePx };
}

/** A label record's second number. */
export const LABEL = { dimension: 0, text: 1, center: 2, corner: 3, beside: 4, along: 5, pieceText: 6, pieceDimension: 7, leader: 8, pieceLeader: 9, line: 10, paragraphMask: 11, pieceLine: 12 } as const;
/**
 * Numbers per label record: `id, what, x, y, a, b, c, d, e`. A text's x, y are where its baseline starts (its
 * point moved by its alignment), b its width factor, c its mask's width (0 none); a block's text piece's d and e
 * (docs/adr/0145). A dimension's c is its unit code, d its prefix code, e 1 for a mask; a block's dimension
 * piece's e too (docs/adr/0147).
 */
export const LABEL_STRIDE = 9;
/*
 * A multi-line text's records (docs/adr/0182 §3): its mask's box (`paragraphMask`: x, y the box's corner under its
 * first letter's left, a the turn, b the box's width along the baseline, c its height up, d a block's piece's place),
 * then a record a line (`line`: x, y where its baseline starts, a the turn, b the height, c the width factor, d and e
 * the line's letters start..end, Unicode scalar values; a block's piece's `pieceLine`: c the piece's place, its width
 * factor the piece's). A leaning text's (docs/adr/0183 §2) lines start further back the lower they are and its mask's
 * corner moves with the slant: the box leans as a whole, each line and the mask from their own baselines.
 */
/** A dimension record's unit code (the core's `DIMENSION_UNITS`). */
export const DIMENSION_UNIT = ['length', 'angle', 'percent', 'coordinate'] as const;
/** A dimension record's prefix code (the core's `DIMENSION_PREFIXES`). */
export const DIMENSION_PREFIX = ['', 'R ', 'Ø ', 'Y=', 'X=', 't=', '%'] as const;

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
