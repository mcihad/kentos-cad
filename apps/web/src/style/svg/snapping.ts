import { bez, bezDeriv, flattenCubic, nearestOnCubic, segmentCount, segmentCubic, segmentIsLine, flattenSubPathTol, ringSignedArea, type Cubic } from './bezier';
import { nodeTypeOf } from './nodeOps';
import type { Box, Pt, SubPath } from './pathData';
import { shapeBox, toPath, type Guide, type SvgShape } from './svgModel';

/**
 * Snapping in the SVG editor, like the main CAD's object snaps: nodes
 * (cusp and smooth), segment middles, crossings of outlines, bounding box
 * corners, edge middles and centres, object centres, the foot of a
 * perpendicular and tangent points (from the point the tool started at),
 * guides, and the canvas border and centre. Fixed points sit in a grid of
 * cells, outlines as chords in another; crossings, perpendiculars and
 * tangents are found only near the pointer. Pure.
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

// Equal distances: the more specific snap wins.
const RANK: Record<SnapKind, number> = { cusp: 0, smooth: 0, intersection: 1, guide: 1, centre: 2, bboxCentre: 3, mid: 4, bboxCorner: 4, page: 5, bboxMid: 5, perpendicular: 6, tangent: 6 };

interface Fixed {
  p: Pt;
  kind: SnapKind;
  label: string;
}

interface Seg {
  c: Cubic;
  line: boolean;
}

interface Chord {
  a: Pt;
  b: Pt;
  seg: number;
}

interface Line {
  p: Pt;
  d: Pt;
  kind: 'guide' | 'page';
  label: string;
  /** A finite line (a canvas edge) runs t ∈ [0, len]; a guide is endless. */
  len?: number;
}

const label = (k: SnapKind) => SNAP_KINDS.find((s) => s.kind === k)!.label;

/** Grid of cells holding item indices by their boxes. */
class Cells {
  private readonly map = new Map<string, number[]>();
  readonly size: number;
  constructor(size: number) {
    this.size = size;
  }
  add(b: Box, i: number): void {
    const s = this.size;
    for (let x = Math.floor(b.minX / s); x <= Math.floor(b.maxX / s); x++)
      for (let y = Math.floor(b.minY / s); y <= Math.floor(b.maxY / s); y++) {
        const k = `${x},${y}`;
        const l = this.map.get(k);
        if (l) l.push(i);
        else this.map.set(k, [i]);
      }
  }
  near(p: Pt, r: number): Set<number> {
    const s = this.size;
    const out = new Set<number>();
    const x0 = Math.floor((p[0] - r) / s);
    const x1 = Math.floor((p[0] + r) / s);
    const y0 = Math.floor((p[1] - r) / s);
    const y1 = Math.floor((p[1] + r) / s);
    // A search wider than the grid is worth: at most a few hundred cells.
    if ((x1 - x0 + 1) * (y1 - y0 + 1) > 400) {
      for (const l of this.map.values()) for (const i of l) out.add(i);
      return out;
    }
    for (let x = x0; x <= x1; x++) for (let y = y0; y <= y1; y++) for (const i of this.map.get(`${x},${y}`) ?? []) out.add(i);
    return out;
  }
}

export class SnapIndex {
  private readonly kinds: ReadonlySet<SnapKind>;
  private readonly fixed: Fixed[] = [];
  private readonly segs: Seg[] = [];
  private readonly chords: Chord[] = [];
  private readonly lines: Line[] = [];
  private readonly fixedCells: Cells;
  private readonly chordCells: Cells;

  constructor(src: SnapSource) {
    this.kinds = src.kinds;
    const { width, height } = src.page;
    const extent = Math.max(width, height, 1e-6);
    this.fixedCells = new Cells(extent / 32);
    this.chordCells = new Cells(extent / 32);
    const tol = extent * 1e-4;
    const on = (k: SnapKind) => this.kinds.has(k);
    const addFixed = (p: Pt, kind: SnapKind, text = label(kind)) => {
      if (!on(kind) || !Number.isFinite(p[0]) || !Number.isFinite(p[1])) return;
      this.fixedCells.add({ minX: p[0], minY: p[1], maxX: p[0], maxY: p[1] }, this.fixed.length);
      this.fixed.push({ p, kind, label: text });
    };
    for (const s of src.shapes) {
      if (s.hidden || src.exclude?.has(s.id)) continue;
      const b = shapeBox(s);
      const cx = (b.minX + b.maxX) / 2;
      const cy = (b.minY + b.maxY) / 2;
      for (const p of [
        [b.minX, b.minY],
        [b.maxX, b.minY],
        [b.minX, b.maxY],
        [b.maxX, b.maxY],
      ] as Pt[])
        addFixed(p, 'bboxCorner');
      for (const p of [
        [cx, b.minY],
        [cx, b.maxY],
        [b.minX, cy],
        [b.maxX, cy],
      ] as Pt[])
        addFixed(p, 'bboxMid');
      addFixed([cx, cy], 'bboxCentre');
      if (s.kind === 'text') {
        addFixed([s.x, s.y], 'centre', 'Yazı başlangıcı');
        continue;
      }
      if (s.kind === 'ellipse') addFixed([s.cx, s.cy], 'centre');
      if (s.kind === 'rect') addFixed([s.x + s.w / 2, s.y + s.h / 2], 'centre');
      const path = toPath(s);
      if (path.kind !== 'path') continue;
      if (s.kind === 'path') {
        const c = centroid(path.subs);
        if (c) addFixed(c, 'centre', 'Ağırlık merkezi');
      }
      path.subs.forEach((sp, si) => {
        const skip = (i: number) => s.kind === 'path' && !!src.skipNode?.(s.id, si, i);
        sp.nodes.forEach((n, i) => {
          if (skip(i)) return;
          const open = !sp.closed && (i === 0 || i === sp.nodes.length - 1);
          addFixed([n.x, n.y], open || nodeTypeOf(sp, i) === 'cusp' ? 'cusp' : 'smooth');
        });
        const count = segmentCount(sp);
        for (let i = 0; i < count; i++) {
          if (skip(i) || skip((i + 1) % sp.nodes.length)) continue;
          const c = segmentCubic(sp, i);
          const line = segmentIsLine(sp, i);
          addFixed(bez(c, 0.5), 'mid');
          const seg = this.segs.length;
          this.segs.push({ c, line });
          const pts = line ? [c[0], c[3]] : flattenCubic(c, tol).pts;
          for (let k = 1; k < pts.length; k++) {
            const a = pts[k - 1];
            const bb = pts[k];
            this.chordCells.add({ minX: Math.min(a[0], bb[0]), minY: Math.min(a[1], bb[1]), maxX: Math.max(a[0], bb[0]), maxY: Math.max(a[1], bb[1]) }, this.chords.length);
            this.chords.push({ a, b: bb, seg });
          }
        }
      });
    }
    // The canvas: corners, edge middles and centre; its edges as lines.
    for (const p of [
      [0, 0],
      [width, 0],
      [0, height],
      [width, height],
    ] as Pt[])
      addFixed(p, 'page', 'Tuval köşesi');
    for (const p of [
      [width / 2, 0],
      [width / 2, height],
      [0, height / 2],
      [width, height / 2],
    ] as Pt[])
      addFixed(p, 'page', 'Tuval kenar ortası');
    addFixed([width / 2, height / 2], 'page', 'Tuval ortası');
    if (on('page')) {
      this.lines.push({ p: [0, 0], d: [1, 0], kind: 'page', label: 'Tuval kenarı', len: width }, { p: [0, height], d: [1, 0], kind: 'page', label: 'Tuval kenarı', len: width });
      this.lines.push({ p: [0, 0], d: [0, 1], kind: 'page', label: 'Tuval kenarı', len: height }, { p: [width, 0], d: [0, 1], kind: 'page', label: 'Tuval kenarı', len: height });
    }
    if (on('guide')) {
      const gl = (src.guides ?? []).map((g): Line => {
        const a = (g.angle * Math.PI) / 180;
        return { p: [g.x, g.y], d: [Math.cos(a), Math.sin(a)], kind: 'guide', label: 'Kılavuz' };
      });
      this.lines.push(...gl);
      // Guides crossing each other.
      for (let i = 0; i < gl.length; i++)
        for (let j = i + 1; j < gl.length; j++) {
          const x = lineCross(gl[i], gl[j]);
          if (x) addFixed(x, 'guide', 'Kılavuz kesişimi');
        }
    }
  }

  /**
   * The best snap within `r` of p. Points (nodes, crossings, centres …)
   * win over lines (guides, canvas edges); `from` is where the tool started,
   * for perpendicular and tangent snaps.
   */
  query(p: Pt, r: number, from?: Pt | null): SnapHit | null {
    let best: SnapHit | null = null;
    const offer = (q: Pt, kind: SnapKind, text: string) => {
      const d = Math.hypot(q[0] - p[0], q[1] - p[1]);
      if (d > r) return;
      if (!best || d < best.d - 1e-9 * r || (Math.abs(d - best.d) <= 1e-9 * r && RANK[kind] < RANK[best.kind])) best = { p: q, kind, label: text, d };
    };
    for (const i of this.fixedCells.near(p, r)) offer(this.fixed[i].p, this.fixed[i].kind, this.fixed[i].label);
    const near = [...this.chordCells.near(p, r)].map((i) => this.chords[i]);
    if (this.kinds.has('intersection')) {
      for (let i = 0; i < near.length; i++)
        for (let j = i + 1; j < near.length; j++) {
          if (near[i].seg === near[j].seg) continue;
          const x = segCross(near[i].a, near[i].b, near[j].a, near[j].b);
          if (x) offer(x, 'intersection', label('intersection'));
        }
      for (const l of this.lines) {
        if (l.kind !== 'guide') continue;
        for (const c of near) {
          const x = segLineCross(c.a, c.b, l);
          if (x) offer(x, 'intersection', 'Kılavuzla kesişim');
        }
      }
    }
    if (from && (this.kinds.has('perpendicular') || this.kinds.has('tangent'))) {
      const segs = new Set(near.map((c) => c.seg));
      for (const s of segs) {
        const seg = this.segs[s];
        if (this.kinds.has('perpendicular')) {
          const q = perpendicularFoot(seg, from, p);
          if (q) offer(q, 'perpendicular', label('perpendicular'));
        }
        if (this.kinds.has('tangent') && !seg.line) {
          const q = tangentPoint(seg.c, from, p);
          if (q) offer(q, 'tangent', label('tangent'));
        }
      }
    }
    if (best) return best;
    // Lines last: the nearest point on a guide or a canvas edge.
    for (const l of this.lines) {
      let t = (p[0] - l.p[0]) * l.d[0] + (p[1] - l.p[1]) * l.d[1];
      if (l.len !== undefined) t = Math.max(0, Math.min(l.len, t));
      offer([l.p[0] + l.d[0] * t, l.p[1] + l.d[1] * t], l.kind, l.label);
    }
    return best;
  }
}

/** Area centroid of the closed sub-paths (flattened); null when none has area. */
function centroid(subs: readonly SubPath[]): Pt | null {
  let a = 0;
  let x = 0;
  let y = 0;
  for (const sp of subs) {
    if (!sp.closed) continue;
    const ring = flattenSubPathTol(sp, 0.01);
    const ra = ringSignedArea(ring);
    if (Math.abs(ra) < 1e-12) continue;
    let cx = 0;
    let cy = 0;
    for (let i = 0; i < ring.length; i++) {
      const p = ring[i];
      const q = ring[(i + 1) % ring.length];
      const k = p[0] * q[1] - q[0] * p[1];
      cx += (p[0] + q[0]) * k;
      cy += (p[1] + q[1]) * k;
    }
    x += cx / 6;
    y += cy / 6;
    a += ra;
  }
  return Math.abs(a) > 1e-12 ? [x / a, y / a] : null;
}

function segCross(a: Pt, b: Pt, c: Pt, d: Pt): Pt | null {
  const rx = b[0] - a[0];
  const ry = b[1] - a[1];
  const sx = d[0] - c[0];
  const sy = d[1] - c[1];
  const den = rx * sy - ry * sx;
  if (Math.abs(den) < 1e-18) return null;
  const t = ((c[0] - a[0]) * sy - (c[1] - a[1]) * sx) / den;
  const u = ((c[0] - a[0]) * ry - (c[1] - a[1]) * rx) / den;
  return t >= 0 && t <= 1 && u >= 0 && u <= 1 ? [a[0] + rx * t, a[1] + ry * t] : null;
}

function segLineCross(a: Pt, b: Pt, l: Line): Pt | null {
  const n: Pt = [-l.d[1], l.d[0]];
  const fa = (a[0] - l.p[0]) * n[0] + (a[1] - l.p[1]) * n[1];
  const fb = (b[0] - l.p[0]) * n[0] + (b[1] - l.p[1]) * n[1];
  if (fa === fb || fa * fb > 0) return null;
  const t = fa / (fa - fb);
  return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
}

function lineCross(a: Line, b: Line): Pt | null {
  const den = a.d[0] * b.d[1] - a.d[1] * b.d[0];
  if (Math.abs(den) < 1e-12) return null;
  const t = ((b.p[0] - a.p[0]) * b.d[1] - (b.p[1] - a.p[1]) * b.d[0]) / den;
  return [a.p[0] + a.d[0] * t, a.p[1] + a.d[1] * t];
}

/** Newton on f(t) = 0 from t0, staying in [0, 1]; null if it does not settle. */
function solve(f: (t: number) => number, t0: number): number | null {
  let t = t0;
  for (let it = 0; it < 30; it++) {
    const v = f(t);
    const h = 1e-7;
    const d = (f(Math.min(1, t + h)) - f(Math.max(0, t - h))) / (Math.min(1, t + h) - Math.max(0, t - h));
    if (!Number.isFinite(d) || Math.abs(d) < 1e-18) return null;
    const next = Math.min(1, Math.max(0, t - v / d));
    if (Math.abs(next - t) < 1e-12) return Math.abs(f(next)) < 1e-6 ? next : null;
    t = next;
  }
  return Math.abs(f(t)) < 1e-6 ? t : null;
}

/** Foot of the perpendicular from `from` onto the segment, the one near p. */
function perpendicularFoot(seg: Seg, from: Pt, p: Pt): Pt | null {
  const c = seg.c;
  if (seg.line) {
    const dx = c[3][0] - c[0][0];
    const dy = c[3][1] - c[0][1];
    const l2 = dx * dx + dy * dy;
    if (l2 < 1e-18) return null;
    const t = ((from[0] - c[0][0]) * dx + (from[1] - c[0][1]) * dy) / l2;
    return t >= 0 && t <= 1 ? [c[0][0] + dx * t, c[0][1] + dy * t] : null;
  }
  const t0 = nearestOnCubic(c, p).t;
  const t = solve((u) => {
    const q = bez(c, u);
    const d = bezDeriv(c, u);
    const l = Math.hypot(d[0], d[1]) || 1;
    return ((q[0] - from[0]) * d[0] + (q[1] - from[1]) * d[1]) / l;
  }, t0);
  return t === null ? null : bez(c, t);
}

/** Point near p where the line from `from` touches the curve. */
function tangentPoint(c: Cubic, from: Pt, p: Pt): Pt | null {
  const t0 = nearestOnCubic(c, p).t;
  const t = solve((u) => {
    const q = bez(c, u);
    const d = bezDeriv(c, u);
    const l = Math.hypot(d[0], d[1]) || 1;
    return ((q[0] - from[0]) * d[1] - (q[1] - from[1]) * d[0]) / l;
  }, t0);
  return t === null ? null : bez(c, t);
}
