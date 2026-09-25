import { readResult, writeArgs } from '../../wasm/core';
import type { Pt } from './pathData';
import { SnapIndex as CoreSnapIndex } from './pkg/kentos_svg_wasm.js';
import type { Guide, SvgShape } from './svgModel';

/**
 * Snapping in the SVG editor, like the main CAD's object snaps: nodes
 * (cusp and smooth), segment middles, crossings of outlines, bounding box
 * corners, edge middles and centres, object centres, the foot of a
 * perpendicular and tangent points (from the point the tool started at),
 * guides, and the canvas border and centre. The index lives in the SVG
 * core (crates/shared/svg-core `snap.rs`): built once per drawing and view,
 * asked on every pointer move.
 */

export type SnapKind = 'cusp' | 'smooth' | 'mid' | 'intersection' | 'bboxCorner' | 'bboxMid' | 'bboxCentre' | 'centre' | 'perpendicular' | 'tangent' | 'guide' | 'page';

/** The kinds in the order the snap bar lists them, with their names. */
export const SNAP_KINDS: readonly { kind: SnapKind; label: string }[] = [
  { kind: 'cusp', label: 'Köşe düğüm' },
  { kind: 'smooth', label: 'Yumuşak düğüm' },
  { kind: 'mid', label: 'Parça ortası' },
  { kind: 'intersection', label: 'Kesişim' },
  { kind: 'bboxCorner', label: 'Kutu köşesi' },
  { kind: 'bboxMid', label: 'Kutu kenar ortası' },
  { kind: 'bboxCentre', label: 'Kutu merkezi' },
  { kind: 'centre', label: 'Nesne merkezi' },
  { kind: 'perpendicular', label: 'Dik' },
  { kind: 'tangent', label: 'Teğet' },
  { kind: 'guide', label: 'Kılavuz' },
  { kind: 'page', label: 'Tuval kenarı ve ortası' },
];

export type { Guide };

export interface SnapHit {
  p: Pt;
  kind: SnapKind;
  label: string;
  d: number;
}

export interface SnapSource {
  shapes: readonly SvgShape[];
  guides?: readonly Guide[];
  page: { width: number; height: number };
  kinds: ReadonlySet<SnapKind>;
  /** Shapes that move with the pointer (never snap to themselves). */
  exclude?: ReadonlySet<string>;
  /** Nodes that move (node dragging): their points and segments are left out. */
  skipNode?: (shape: string, sub: number, index: number) => boolean;
}

export class SnapIndex {
  private readonly core: CoreSnapIndex;

  constructor(src: SnapSource) {
    // The node filter crosses as the list of nodes it leaves out.
    const skip: [string, number, number][] = [];
    const skipNode = src.skipNode;
    if (skipNode) for (const s of src.shapes) if (s.kind === 'path') s.subs.forEach((sp, si) => sp.nodes.forEach((_n, i) => (skipNode(s.id, si, i) ? skip.push([s.id, si, i]) : undefined)));
    this.core = CoreSnapIndex.of(writeArgs({ shapes: src.shapes, guides: src.guides ?? [], page: src.page, kinds: [...src.kinds], exclude: [...(src.exclude ?? [])], skip }));
  }

  /**
   * The best snap within `r` of p. Points (nodes, crossings, centres …)
   * win over lines (guides, canvas edges); `from` is where the tool started,
   * for perpendicular and tangent snaps.
   */
  query(p: Pt, r: number, from?: Pt | null): SnapHit | null {
    return readResult(this.core.query(p[0], p[1], r, !!from, from ? from[0] : 0, from ? from[1] : 0)) as SnapHit | null;
  }
}
