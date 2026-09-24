import type { Vec2 } from '../../model/geometry';
import { TAU } from '../../model/geom/arc';
import type { Edge } from '../../model/geom/intersect';
import { overlay, type Source } from '../../model/geom/overlay';
import { areaSource } from '../../model/geom/region';
import { bezTangent, flattenCubic, ringSignedArea, segmentCount, segmentCubic, segmentIsLine } from './bezier';
import { areasToSubPaths, extentOf, regionSource, tolFor, Tracer, type RegionInput } from './pathBool';
import type { Pt, SubPath } from './pathData';

/**
 * Stroke outlines and offsets as fill regions. A stroke is the union of
 * simple pieces, all counter-clockwise in one overlay source (so nonzero
 * winding is their union): a ribbon along every smooth run of the
 * flattened path (per chord rectangles where the inner side folds), a join
 * at every corner (round disc, mitre within the limit, else bevel) and a
 * cap at every open end (round disc, square box, nothing for butt). Dashes
 * cut the path first. Inset and outset take the stroke of the region's
 * outline away from it or add it. The union's outline is fitted with
 * cubics; round joins and caps are exact arcs turned into cubics.
 */

export type Cap = 'butt' | 'round' | 'square';
export type Join = 'miter' | 'round' | 'bevel';

export interface StrokeStyle {
  width: number;
  cap: Cap;
  join: Join;
  /** SVG stroke-miterlimit (default 4). */
  miterLimit?: number;
  dash?: readonly number[];
}

/**
 * A flattened line: its points, whether each one is a corner (a join goes
 * there) and whether it is a node of the source (a sample inside a curve
 * turning sharply gets a round join whatever the style: a smooth curve's
 * stroke has no corners).
 */
interface Poly {
  pts: Pt[];
  corner: boolean[];
  node: boolean[];
  closed: boolean;
}

const v = (p: Pt): Vec2 => ({ x: p[0], y: p[1] });
const sub = (a: Pt, b: Pt): Pt => [a[0] - b[0], a[1] - b[1]];
const unit = (a: Pt): Pt => {
  const l = Math.hypot(a[0], a[1]);
  return l > 1e-15 ? [a[0] / l, a[1] / l] : [0, 0];
};
/** Left normal of the direction a→b (x right, y as given). */
const normal = (a: Pt, b: Pt): Pt => {
  const d = unit(sub(b, a));
  return [-d[1], d[0]];
};
const cross = (a: Pt, b: Pt) => a[0] * b[1] - a[1] * b[0];

/** Turns sharper than this (radians) between chords are corners that get the join. */
const CORNER = (12 * Math.PI) / 180;

function polyOf(sp: SubPath, tol: number): Poly {
  const pts: Pt[] = [];
  const node: boolean[] = [];
  const push = (p: Pt, isNode: boolean) => {
    const q = pts[pts.length - 1];
    if (q && Math.hypot(p[0] - q[0], p[1] - q[1]) < 1e-12) {
      node[node.length - 1] ||= isNode;
      return;
    }
    pts.push(p);
    node.push(isNode);
  };
  if (!sp.nodes.length) return { pts, corner: [], node, closed: sp.closed };
  const n = segmentCount(sp);
  // A node where the tangent runs on (a smooth node) is no corner: its stroke needs no join.
  const cornerNode = (i: number) => {
    if (!sp.closed && (i === 0 || i === sp.nodes.length - 1)) return true;
    if (n < 2) return true;
    const a = bezTangent(segmentCubic(sp, (i - 1 + n) % n), 1);
    const b = bezTangent(segmentCubic(sp, i % n), 0);
    return a[0] * b[0] + a[1] * b[1] < Math.cos((0.5 * Math.PI) / 180);
  };
  push([sp.nodes[0].x, sp.nodes[0].y], cornerNode(0));
  for (let i = 0; i < n; i++) {
    const c = segmentCubic(sp, i);
    const end = cornerNode(i + 1);
    if (segmentIsLine(sp, i)) push(c[3], end);
    else {
      const { pts: ps } = flattenCubic(c, tol);
      for (let k = 1; k < ps.length; k++) push(ps[k], k === ps.length - 1 && end);
    }
  }
  if (sp.closed && pts.length > 1) {
    const a = pts[0];
    const b = pts[pts.length - 1];
    if (Math.hypot(a[0] - b[0], a[1] - b[1]) < 1e-12) {
      pts.pop();
      node.pop();
    }
  }
  return withCorners(pts, node, sp.closed, 1e-3);
}

/** Corners: nodes where the line turns more than `nodeTurn` (radians), and sharp turns anywhere. */
function withCorners(pts: Pt[], node: boolean[], closed: boolean, nodeTurn: number): Poly {
  const m = pts.length;
  const corner = node.map(() => false);
  for (let i = 0; i < m; i++) {
    if (!closed && (i === 0 || i === m - 1)) continue;
    const a = pts[(i - 1 + m) % m];
    const b = pts[i];
    const c = pts[(i + 1) % m];
    const turn = Math.abs(Math.atan2(cross(sub(b, a), sub(c, b)), (b[0] - a[0]) * (c[0] - b[0]) + (b[1] - a[1]) * (c[1] - b[1])));
    corner[i] = turn > CORNER || (node[i] && turn > nodeTurn);
  }
  return { pts, corner, node, closed };
}

/** A line cut into dashes (on, off … repeated; an odd list is doubled as SVG does). */
function dashed(p: Poly, pattern: readonly number[]): Poly[] {
  const pat = pattern.length % 2 ? [...pattern, ...pattern] : [...pattern];
  const total = pat.reduce((s, x) => s + x, 0);
  if (!(total > 0) || pat.some((x) => x < 0)) return [p];
  const pts = p.closed ? [...p.pts, p.pts[0]] : p.pts;
  const corner = p.closed ? [...p.corner, p.corner[0]] : p.corner;
  const node = p.closed ? [...p.node, p.node[0]] : p.node;
  const out: Poly[] = [];
  let k = 0;
  let left = pat[0];
  let on = true;
  const start = (q: Pt): Poly => ({ pts: [q], corner: [false], node: [true], closed: false });
  let cur: Poly | null = start(pts[0]);
  for (let i = 1; i < pts.length; i++) {
    let a = pts[i - 1];
    const b = pts[i];
    let seg = Math.hypot(b[0] - a[0], b[1] - a[1]);
    while (seg > left) {
      const t = left / seg;
      const q: Pt = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
      if (on && cur) {
        cur.pts.push(q);
        cur.corner.push(false);
        cur.node.push(true);
        out.push(cur);
        cur = null;
      } else cur = start(q);
      on = !on;
      seg -= left;
      a = q;
      k = (k + 1) % pat.length;
      left = pat[k];
    }
    left -= seg;
    if (on && cur) {
      cur.pts.push(b);
      cur.corner.push(corner[i]);
      cur.node.push(node[i]);
    }
  }
  if (on && cur && cur.pts.length > 1) out.push(cur);
  return out.filter((d) => d.pts.length >= 2 || pat[0] === 0);
}

/** Collects the stroke's pieces (all counter-clockwise) into one source. */
class Pieces {
  readonly edges: Edge[] = [];
  readonly points: Vec2[] = [];

  poly(ps: readonly Pt[]): void {
    if (ps.length < 3) return;
    const area = ringSignedArea(ps);
    if (Math.abs(area) < 1e-18) return;
    const ring = area > 0 ? ps : [...ps].reverse();
    for (let i = 0; i < ring.length; i++) this.edges.push({ kind: 'seg', a: v(ring[i]), b: v(ring[(i + 1) % ring.length]) });
    this.points.push(...ring.map(v));
  }

  disc(c: Pt, r: number): void {
    if (r > 0) this.edges.push({ kind: 'arc', c: v(c), r, a0: 0, sweep: TAU });
  }

  get source(): Source {
    return { edges: this.edges, points: this.points };
  }
}

function joinAt(out: Pieces, p: Pt, a: Pt, b: Pt, h: number, st: StrokeStyle): void {
  const n1 = normal(a, p);
  const n2 = normal(p, b);
  const turn = cross(sub(p, a), sub(b, p));
  if (Math.abs(turn) < 1e-18 && n1[0] * n2[0] + n1[1] * n2[1] > 0) return;
  if (st.join === 'round') return out.disc(p, h);
  // The outer side is the right side when the path turns left.
  const s = turn > 0 ? -1 : 1;
  const A: Pt = [p[0] + s * n1[0] * h, p[1] + s * n1[1] * h];
  const B: Pt = [p[0] + s * n2[0] * h, p[1] + s * n2[1] * h];
  if (st.join === 'miter') {
    const cosT = n1[0] * n2[0] + n1[1] * n2[1];
    // SVG: miter length / stroke width = 1 / sin(θ/2), θ the angle between the segments.
    const ratio = 1 / Math.sqrt(Math.max(1e-18, (1 + cosT) / 2));
    if (ratio <= (st.miterLimit ?? 4)) {
      const k = h / (1 + cosT);
      const M: Pt = [p[0] + s * (n1[0] + n2[0]) * k, p[1] + s * (n1[1] + n2[1]) * k];
      return out.poly([p, A, M, B]);
    }
  }
  out.poly([p, A, B]);
}

function capAt(out: Pieces, p: Pt, towards: Pt, h: number, cap: Cap): void {
  if (cap === 'round') return out.disc(p, h);
  if (cap !== 'square') return;
  const d = unit(sub(p, towards));
  const n: Pt = [-d[1], d[0]];
  const e: Pt = [p[0] + d[0] * h, p[1] + d[1] * h];
  out.poly([
    [p[0] + n[0] * h, p[1] + n[1] * h],
    [e[0] + n[0] * h, e[1] + n[1] * h],
    [e[0] - n[0] * h, e[1] - n[1] * h],
    [p[0] - n[0] * h, p[1] - n[1] * h],
  ]);
}

/**
 * A smooth run as ribbons: offset points on both sides (mitred between
 * chords), grown chord by chord while neither side folds back (a turn
 * tighter than the half width). Where one would fold, the ribbon ends and
 * a new one starts, a disc filling the gap. So a gentle curve is one
 * polygon and the overlay stays small.
 */
function runAt(out: Pieces, pts: readonly Pt[], h: number): void {
  const k = pts.length;
  if (k < 2) return;
  const nrm: Pt[] = [];
  for (let i = 1; i < k; i++) nrm.push(normal(pts[i - 1], pts[i]));
  // Offset direction at point i of a ribbon from s to e (plain normals at its ends).
  const dir = (i: number, s: number, e: number): Pt => {
    if (i === s) return nrm[s];
    if (i === e) return nrm[e - 1];
    const n1 = nrm[i - 1];
    const n2 = nrm[i];
    const d = 1 + n1[0] * n2[0] + n1[1] * n2[1];
    return d > 1e-6 ? [(n1[0] + n2[0]) / d, (n1[1] + n2[1]) / d] : n1;
  };
  // Chord i−1 → i keeps going forward on both sides.
  const forward = (i: number, s: number, e: number) => {
    const a = dir(i - 1, s, e);
    const b = dir(i, s, e);
    const dx = pts[i][0] - pts[i - 1][0];
    const dy = pts[i][1] - pts[i - 1][1];
    const l = dx + (b[0] - a[0]) * h;
    const lY = dy + (b[1] - a[1]) * h;
    const r = dx - (b[0] - a[0]) * h;
    const rY = dy - (b[1] - a[1]) * h;
    return l * dx + lY * dy > 0 && r * dx + rY * dy > 0;
  };
  let s = 0;
  while (s < k - 1) {
    let e = s + 1;
    while (e + 1 <= k - 1 && forward(e, s, e + 1) && forward(e + 1, s, e + 1)) e++;
    const L: Pt[] = [];
    const R: Pt[] = [];
    for (let i = s; i <= e; i++) {
      const m = dir(i, s, e);
      L.push([pts[i][0] + m[0] * h, pts[i][1] + m[1] * h]);
      R.push([pts[i][0] - m[0] * h, pts[i][1] - m[1] * h]);
    }
    // Every quad of an unfolded ribbon turns the same way, so its winding counts the quads covering a point.
    out.poly([...L, ...R.reverse()]);
    if (e < k - 1) out.disc(pts[e], h);
    s = e;
  }
}

function strokePoly(out: Pieces, p: Poly, h: number, style: StrokeStyle): void {
  const pts = p.pts;
  const round: StrokeStyle = { ...style, join: 'round' };
  const st = style;
  const joinStyle = (i: number) => (p.node[i] ? st : round);
  const n = pts.length;
  if (n === 1) {
    // A zero-length sub-path: SVG draws a dot for round and square caps.
    if (st.cap === 'round') out.disc(pts[0], h);
    else if (st.cap === 'square') out.poly([[pts[0][0] - h, pts[0][1] - h], [pts[0][0] + h, pts[0][1] - h], [pts[0][0] + h, pts[0][1] + h], [pts[0][0] - h, pts[0][1] + h]]);
    return;
  }
  const corners: number[] = [];
  for (let i = 0; i < n; i++) if (p.corner[i] && (p.closed || (i > 0 && i < n - 1))) corners.push(i);
  if (p.closed) {
    if (!corners.length) {
      // A smooth loop: one run all round, closed by repeating the start.
      runAt(out, [...pts, pts[0], pts[1 % n]], h);
      return;
    }
    for (let c = 0; c < corners.length; c++) {
      const i0 = corners[c];
      const i1 = c === corners.length - 1 ? corners[0] + n : corners[c + 1];
      const run: Pt[] = [];
      for (let i = i0; i <= i1; i++) run.push(pts[i % n]);
      runAt(out, run, h);
      joinAt(out, pts[i0], pts[(i0 - 1 + n) % n], pts[(i0 + 1) % n], h, joinStyle(i0));
    }
    return;
  }
  const cuts = [0, ...corners, n - 1];
  for (let c = 0; c + 1 < cuts.length; c++) runAt(out, pts.slice(cuts[c], cuts[c + 1] + 1), h);
  for (const i of corners) joinAt(out, pts[i], pts[i - 1], pts[i + 1], h, joinStyle(i));
  capAt(out, pts[0], pts[1], h, st.cap);
  capAt(out, pts[n - 1], pts[n - 2], h, st.cap);
}

/** Pieces of the stroke of these sub-paths (one overlay source). */
export function strokeSource(subs: readonly SubPath[], st: StrokeStyle, tol: number): Source {
  const out = new Pieces();
  const h = st.width / 2;
  if (!(h > 0)) return out.source;
  for (const sp of subs) {
    const p = polyOf(sp, tol);
    if (!p.pts.length) continue;
    const parts = st.dash?.length && st.dash.some((d) => d > 0) ? dashed(p, st.dash) : [p];
    for (const part of parts) strokePoly(out, part, h, st);
  }
  return out.source;
}

/**
 * Fit tolerance for outlines: a few times the flattening's (so a quarter
 * circle fits one cubic), never more than a small part of the stroke width
 * or offset (`size`), where a wobble would show.
 */
const fitTolFor = (tol: number, size: number) => Math.min(tol * 8, Math.max(size * 0.02, tol * 2));

/** The outline of a stroke as closed sub-paths (the area the stroke paints). */
export function strokeOutline(subs: readonly SubPath[], st: StrokeStyle): SubPath[] {
  const extent = extentOf([subs]) + st.width;
  // Relative to the width (the outline's detail), never coarser than a thousandth of the drawing.
  const tol = Math.min(extent * 1e-3, Math.max(st.width * 0.004, tolFor(extent)));
  const areas = overlay([strokeSource(subs, st, tol)], 'first');
  return areasToSubPaths(areas, null, fitTolFor(tol, st.width));
}

/**
 * The fill region grown (d > 0) or shrunk (d < 0) by |d|: its outline's
 * stroke of width 2|d| added or taken away. Round joins round the corners
 * that open up (Inkscape's outset), mitre joins keep them sharp.
 */
export function offsetRegion(input: RegionInput, d: number, join: Join = 'round'): SubPath[] {
  if (!d) return input.subs.map((sp) => ({ closed: sp.closed, nodes: sp.nodes.map((n) => ({ ...n })) }));
  const extent = extentOf([input.subs]) + 2 * Math.abs(d);
  const tol = Math.min(tolFor(extent), Math.max(Math.abs(d) * 0.01, 1e-7));
  const tr = new Tracer(tol, extent);
  const clean = overlay([regionSource(input, tr)], 'first');
  if (!clean.length) return [];
  // The clean outline, with samples inside curves told apart from real corners.
  const band = new Pieces();
  const st: StrokeStyle = { width: 2 * Math.abs(d), cap: 'butt', join, miterLimit: 8 };
  for (const r of clean.flatMap((a) => [a.outer, ...a.holes])) {
    const pts: Pt[] = r.pts.map((q) => [q.x, q.y]);
    // Input nodes on a curve turn by the sampling angle: only clear turns get the join there.
    strokePoly(band, withCorners(pts, pts.map((q) => !tr.isSample(q)), true, (8 * Math.PI) / 180), Math.abs(d), st);
  }
  const grown = overlay([areaSource(clean), band.source], d > 0 ? 'any' : 'firstNotOthers');
  return areasToSubPaths(grown, tr, fitTolFor(tol, 2 * Math.abs(d)));
}
