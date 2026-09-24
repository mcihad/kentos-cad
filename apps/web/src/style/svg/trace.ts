import { fitRun } from './fitCurve';
import type { PathNode, SubPath } from './pathData';

/**
 * Trace bitmap (Inkscape's, simplified): a raster picture into filled
 * paths with holes, for redrawing the regulation's raster pictograms.
 * Brightness threshold → ink mask; marching squares give the outlines on
 * the pixel lattice with ink always on the same side, so outer rings and
 * holes differ by the sign of their area; rings are nested into outers
 * with their holes and specks below an area go; Douglas–Peucker
 * picks the corners (chamfered pixel corners are recovered first) and
 * the runs between them are fitted with cubics (`fitCurve`). Pure:
 * coordinates are pixels from the image's top-left corner.
 */

/** RGBA pixels, row by row (ImageData or the like). */
export interface Bitmap {
  width: number;
  height: number;
  data: ArrayLike<number>;
}

export interface TraceOptions {
  /** 0–255: pixels darker than this are ink. */
  threshold: number;
  /** Light pixels are ink instead. */
  invert: boolean;
  /** Specks and pinholes smaller than this (px²) are dropped. */
  speckle: number;
  /** Douglas–Peucker tolerance (px). */
  tolerance: number;
  /** Turns sharper than this (degrees) stay corners. */
  corner: number;
  /** 0: straight edges only; above 0: fitted curves, looser (fewer nodes, rounder) as it grows to 1. */
  smooth: number;
}

export const TRACE_DEFAULTS: TraceOptions = { threshold: 128, invert: false, speckle: 6, tolerance: 1, corner: 60, smooth: 1 };

export type Ring = [number, number][];

export interface TracedShape {
  outer: SubPath;
  holes: SubPath[];
}

export interface TraceResult {
  shapes: TracedShape[];
  nodes: number;
  holes: number;
  /** Specks and pinholes dropped. */
  removed: number;
}

/** Ink (1) where the pixel, laid on white paper, is darker than the threshold. */
export function inkMask(img: Bitmap, threshold: number, invert = false): Uint8Array {
  const n = img.width * img.height;
  const out = new Uint8Array(n);
  const d = img.data;
  for (let i = 0; i < n; i++) {
    const a = (d[i * 4 + 3] ?? 255) / 255;
    const y = 0.299 * d[i * 4] + 0.587 * d[i * 4 + 1] + 0.114 * d[i * 4 + 2];
    const lum = 255 - a * (255 - y);
    out[i] = (lum < threshold) !== invert ? 1 : 0;
  }
  return out;
}

/**
 * Outlines of the ink by marching squares on pixel centres: every ring
 * closes, ink lies on the same side of every segment (outer rings have a
 * positive shoelace area in image coordinates, holes a negative one), and
 * diagonal pixels of ink connect.
 */
export function traceContours(mask: ArrayLike<number>, w: number, h: number): Ring[] {
  const at = (i: number, j: number) => (i >= 1 && j >= 1 && i <= w && j <= h ? mask[(j - 1) * w + (i - 1)] : 0);
  // Crossing points in doubled coordinates (2x + 1, 2y + 1) as one number.
  const S = 2 * h + 4;
  const key = (x2: number, y2: number) => x2 * S + y2;
  const succ = new Map<number, number>();
  const seg = (p: number, q: number) => succ.set(p, q);
  for (let j = 0; j <= h; j++) {
    for (let i = 0; i <= w; i++) {
      const tl = at(i, j);
      const tr = at(i + 1, j);
      const br = at(i + 1, j + 1);
      const bl = at(i, j + 1);
      const code = (tl << 3) | (tr << 2) | (br << 1) | bl;
      if (code === 0 || code === 15) continue;
      const top = key(2 * i + 1, 2 * j);
      const right = key(2 * i + 2, 2 * j + 1);
      const bottom = key(2 * i + 1, 2 * j + 2);
      const left = key(2 * i, 2 * j + 1);
      // Each case as (from, to) with the ink on the positive side; saddles connect the ink.
      switch (code) {
        case 1: seg(left, bottom); break; // bl
        case 2: seg(bottom, right); break; // br
        case 3: seg(left, right); break; // bl br
        case 4: seg(right, top); break; // tr
        case 5: seg(left, top); seg(right, bottom); break; // tr bl (saddle)
        case 6: seg(bottom, top); break; // tr br
        case 7: seg(left, top); break; // tr br bl
        case 8: seg(top, left); break; // tl
        case 9: seg(top, bottom); break; // tl bl
        case 10: seg(top, right); seg(bottom, left); break; // tl br (saddle)
        case 11: seg(top, right); break; // tl br bl
        case 12: seg(right, left); break; // tl tr
        case 13: seg(right, bottom); break; // tl tr bl
        case 14: seg(bottom, left); break; // tl tr br
      }
    }
  }
  const rings: Ring[] = [];
  for (const [start] of succ) {
    if (!succ.has(start)) continue;
    const ring: Ring = [];
    let k: number | undefined = start;
    while (k !== undefined) {
      ring.push([(Math.floor(k / S) - 1) / 2, ((k % S) - 1) / 2]);
      const next: number | undefined = succ.get(k);
      succ.delete(k);
      if (next === start) break;
      k = next;
    }
    if (ring.length >= 3) rings.push(ring);
  }
  return rings;
}

/** Signed shoelace area (image coordinates, y down): outer rings positive. */
export function ringArea(r: readonly (readonly [number, number])[]): number {
  let a = 0;
  for (let i = 0, n = r.length; i < n; i++) {
    const p = r[i];
    const q = r[(i + 1) % n];
    a += p[0] * q[1] - q[0] * p[1];
  }
  return a / 2;
}

function inside(p: readonly [number, number], r: Ring): boolean {
  let hit = false;
  for (let i = 0, j = r.length - 1; i < r.length; j = i++) {
    const [xi, yi] = r[i];
    const [xj, yj] = r[j];
    if (yi > p[1] !== yj > p[1] && p[0] < ((xj - xi) * (p[1] - yi)) / (yj - yi) + xi) hit = !hit;
  }
  return hit;
}

/** Outer rings with the holes they hold (the smallest outer around each hole); rings under `minArea` go. */
export function nestRings(rings: Ring[], minArea = 0): { shapes: { outer: Ring; holes: Ring[] }[]; removed: number } {
  let removed = 0;
  const outers: { r: Ring; a: number; box: number[]; holes: Ring[] }[] = [];
  const holes: { r: Ring; a: number }[] = [];
  const boxOf = (r: Ring) => {
    let x0 = Infinity;
    let y0 = Infinity;
    let x1 = -Infinity;
    let y1 = -Infinity;
    for (const [x, y] of r) {
      x0 = Math.min(x0, x);
      y0 = Math.min(y0, y);
      x1 = Math.max(x1, x);
      y1 = Math.max(y1, y);
    }
    return [x0, y0, x1, y1];
  };
  for (const r of rings) {
    const a = ringArea(r);
    if (Math.abs(a) < Math.max(minArea, 1e-9)) {
      removed++;
      continue;
    }
    if (a > 0) outers.push({ r, a, box: boxOf(r), holes: [] });
    else holes.push({ r, a: -a });
  }
  outers.sort((p, q) => p.a - q.a);
  for (const hole of holes) {
    const p = hole.r[0];
    const owner = outers.find((o) => o.a > hole.a && p[0] >= o.box[0] && p[0] <= o.box[2] && p[1] >= o.box[1] && p[1] <= o.box[3] && inside(p, o.r));
    if (owner) owner.holes.push(hole.r);
    else removed++;
  }
  return { shapes: outers.sort((p, q) => q.a - p.a).map((o) => ({ outer: o.r, holes: o.holes })), removed };
}

function segDist(p: readonly [number, number], a: readonly [number, number], b: readonly [number, number]): number {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const l2 = dx * dx + dy * dy;
  const t = l2 ? Math.max(0, Math.min(1, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2)) : 0;
  return Math.hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy);
}

/** Douglas–Peucker on an open chain `a..b` of a ring (both ends kept): the kept indices. */
function dpChain(r: Ring, idx: number[], tol: number): number[] {
  if (idx.length < 3) return idx.slice();
  const keep = new Uint8Array(idx.length);
  keep[0] = keep[idx.length - 1] = 1;
  const stack: [number, number][] = [[0, idx.length - 1]];
  while (stack.length) {
    const [s, e] = stack.pop()!;
    let best = -1;
    let bestD = tol;
    for (let i = s + 1; i < e; i++) {
      const d = segDist(r[idx[i]], r[idx[s]], r[idx[e]]);
      if (d > bestD) {
        bestD = d;
        best = i;
      }
    }
    if (best >= 0) {
      keep[best] = 1;
      stack.push([s, best], [best, e]);
    }
  }
  return idx.filter((_, i) => keep[i]);
}

/** Douglas–Peucker on a closed ring, anchored at two extreme points (real corners, not the arbitrary start): kept indices in ring order. */
export function simplifyIndices(r: Ring, tol: number): number[] {
  const n = r.length;
  if (n < 4) return r.map((_, i) => i);
  let i0 = 0;
  for (let i = 1; i < n; i++) if (r[i][0] + r[i][1] < r[i0][0] + r[i0][1]) i0 = i;
  let i1 = i0;
  let far = -1;
  for (let i = 0; i < n; i++) {
    const d = Math.hypot(r[i][0] - r[i0][0], r[i][1] - r[i0][1]);
    if (d > far) {
      far = d;
      i1 = i;
    }
  }
  const chain = (a: number, b: number) => {
    const out: number[] = [];
    for (let i = a; ; i = (i + 1) % n) {
      out.push(i);
      if (i === b) break;
    }
    return out;
  };
  const t = Math.max(tol, 1e-6);
  const kept = [...dpChain(r, chain(i0, i1), t), ...dpChain(r, chain(i1, i0), t).slice(1, -1)];
  return kept.sort((a, b) => a - b);
}

export const simplifyRing = (r: Ring, tol: number): Ring => simplifyIndices(r, tol).map((i) => r[i]);

const turn = (a: readonly [number, number], b: readonly [number, number], c: readonly [number, number]) => {
  const u = Math.atan2(b[1] - a[1], b[0] - a[0]);
  const v = Math.atan2(c[1] - b[1], c[0] - b[0]);
  return Math.abs(Math.atan2(Math.sin(v - u), Math.cos(v - u)));
};

/** Points where the ring goes straight on are dropped (a run along pixel edges or diagonals becomes one edge). */
export function compactRing(r: Ring): Ring {
  const n = r.length;
  const out: Ring = [];
  for (let i = 0; i < n; i++) {
    const p = r[(i - 1 + n) % n];
    const a = r[i];
    const q = r[(i + 1) % n];
    if (Math.abs((a[0] - p[0]) * (q[1] - a[1]) - (a[1] - p[1]) * (q[0] - a[0])) > 1e-12) out.push(a);
  }
  return out.length >= 3 ? out : r.slice();
}

/**
 * Sharp corners back from the pixel lattice: marching squares cut a
 * square corner with one short diagonal. Where such an edge joins two
 * straight runs of at least `minRun` that turn by `cornerRad` or more
 * together, its ends become the runs' crossing. Staircases of curves
 * (short runs) are left alone.
 */
export function recoverCorners(r: Ring, minRun: number, cornerRad: number): Ring {
  const n = r.length;
  if (n < 4) return r.slice();
  const out: Ring = [];
  const skip = new Uint8Array(n);
  for (let i = 0; i < n; i++) {
    if (skip[i]) continue;
    const a = r[i];
    const b = r[(i + 1) % n];
    const p = r[(i - 1 + n) % n];
    const q = r[(i + 2) % n];
    const len = Math.hypot(b[0] - a[0], b[1] - a[1]);
    const d1 = [a[0] - p[0], a[1] - p[1]];
    const d2 = [q[0] - b[0], q[1] - b[1]];
    const wraps = (i + 1) % n === 0 && out.length === 0;
    if (len < 0.75 && Math.hypot(d1[0], d1[1]) >= minRun && Math.hypot(d2[0], d2[1]) >= minRun && !wraps && !skip[(i + 1) % n]) {
      const den = d1[0] * d2[1] - d1[1] * d2[0];
      const total = Math.abs(Math.atan2(den, d1[0] * d2[0] + d1[1] * d2[1]));
      if (Math.abs(den) > 1e-12 && total >= cornerRad) {
        const t = ((b[0] - a[0]) * d2[1] - (b[1] - a[1]) * d2[0]) / den;
        out.push([a[0] + d1[0] * t, a[1] + d1[1] * t]);
        skip[(i + 1) % n] = 1;
        // The ring's first point may have been this corner's second end.
        if ((i + 1) % n === 0) out.shift();
        continue;
      }
    }
    out.push(a);
  }
  return out.length >= 3 ? out : r.slice();
}

/** Neighbour averaging of the points that are not corners (staircase noise out of curves). */
function smoothRing(r: Ring, fixed: ReadonlySet<number>, passes: number): Ring {
  let cur = r;
  const n = r.length;
  for (let k = 0; k < passes; k++) {
    cur = cur.map((v, i) => (fixed.has(i) ? v : [(cur[(i - 1 + n) % n][0] + 2 * v[0] + cur[(i + 1) % n][0]) / 4, (cur[(i - 1 + n) % n][1] + 2 * v[1] + cur[(i + 1) % n][1]) / 4]));
  }
  return cur;
}

const round = (v: number) => Math.round(v * 1000) / 1000;
const roundNode = (n: PathNode): PathNode => ({ x: round(n.x), y: round(n.y), ...(n.in ? { in: [round(n.in[0]), round(n.in[1])] as const } : {}), ...(n.out ? { out: [round(n.out[0]), round(n.out[1])] as const } : {}) });

/**
 * One traced ring as a closed path. Douglas–Peucker picks the vertices;
 * those that turn by `cornerDeg` or more are corners. Without curves the
 * vertices are the path. With curves the ring's own points (lightly
 * smoothed) are fitted with cubics within the tolerance (loosened by
 * `smooth`), run by run: runs
 * end at corners and, on long bends, about every quarter turn, where both
 * runs share one tangent so the join stays smooth.
 */
export function fitTracedRing(dense: Ring, o: TraceOptions): SubPath | null {
  const cornerRad = (o.corner * Math.PI) / 180;
  const r = recoverCorners(compactRing(dense), 1.5, Math.min(cornerRad, Math.PI / 3));
  const idx = simplifyIndices(r, o.tolerance);
  // Too small for the tolerance (a speck kept on purpose): its own outline.
  const kept = idx.length >= 3 ? idx : r.map((_, i) => i);
  if (kept.length < 3) return null;
  if (o.smooth <= 0) return { closed: true, nodes: kept.map((i) => ({ x: round(r[i][0]), y: round(r[i][1]) })) };
  // Corners are judged on a coarser outline: small tolerances keep staircase steps that are no corners.
  const coarse = o.tolerance >= 1.2 ? kept : simplifyIndices(r, 1.2);
  const cs = coarse.length >= 3 ? coarse : kept;
  const m = cs.length;
  const n = r.length;
  const turns = cs.map((i, k) => turn(r[cs[(k - 1 + m) % m]], r[i], r[cs[(k + 1) % m]]));
  // The point where the ring about 2.5 px away lies, backwards or forwards.
  const away = (i: number, step: number) => {
    let j = i;
    for (let k = 0; k < n; k++) {
      j = (j + step + n) % n;
      if (Math.hypot(r[j][0] - r[i][0], r[j][1] - r[i][1]) >= 2.5) break;
    }
    return r[j];
  };
  // A corner turns sharply both on the outline and right at the point (a coarse outline of a bend does not).
  const corner = new Set(cs.filter((i, k) => turns[k] >= cornerRad && turn(away(i, -1), r[i], away(i, 1)) >= cornerRad * 0.8));
  // Soft splits: a quarter turn gathered since the last split.
  const splits: number[] = [];
  const firstK = cs.findIndex((i) => corner.has(i));
  const startK = firstK >= 0 ? firstK : 0;
  let gathered = 0;
  for (let j = 0; j < m; j++) {
    const k = (startK + j) % m;
    const i = cs[k];
    if (j === 0 || corner.has(i) || gathered + turns[k] > Math.PI / 2) {
      splits.push(i);
      gathered = 0;
    } else gathered += turns[k];
  }
  // One light pass takes the staircase out; more would shrink the shape.
  const pts = smoothRing(r, corner, 1);
  const unit = (x: number, y: number): [number, number] => {
    const l = Math.hypot(x, y) || 1;
    return [x / l, y / l];
  };
  // The tangent at a smooth split, over about 2 px either side.
  const tangent = (i: number): [number, number] => {
    let a = i;
    let b = i;
    for (let k = 0, d = 0; k < n && d < 2; k++) {
      a = (a - 1 + n) % n;
      d = Math.hypot(pts[a][0] - pts[i][0], pts[a][1] - pts[i][1]);
    }
    for (let k = 0, d = 0; k < n && d < 2; k++) {
      b = (b + 1) % n;
      d = Math.hypot(pts[b][0] - pts[i][0], pts[b][1] - pts[i][1]);
    }
    return unit(pts[b][0] - pts[a][0], pts[b][1] - pts[a][1]);
  };
  // Smoothing loosens the fit: fewer nodes, rounder curves.
  const tol = Math.max(o.tolerance, 0.05) * (0.6 + 0.8 * Math.min(1, o.smooth));
  const nodes: PathNode[] = [];
  for (let s = 0; s < splits.length; s++) {
    const i0 = splits[s];
    const i1 = splits[(s + 1) % splits.length];
    const run: [number, number][] = [];
    for (let i = i0; ; i = (i + 1) % n) {
      run.push(pts[i]);
      if (i === i1 && run.length > 1) break;
    }
    const t0 = corner.has(i0) ? undefined : tangent(i0);
    const t1 = corner.has(i1) ? undefined : tangent(i1);
    if (!nodes.length) nodes.push({ x: pts[i0][0], y: pts[i0][1] });
    let curves = fitRun(run, tol, t0, t1 ? [-t1[0], -t1[1]] : undefined);
    if (curves.length === 1 && !curves[0] && (t0 || t1) && run.length > 2) {
      // Straight within the tolerance, but a smooth join must not kink: one cubic along the tangents.
      const a = run[0];
      const b = run[run.length - 1];
      const k = Math.hypot(b[0] - a[0], b[1] - a[1]) / 3;
      const u = t0 ?? unit(b[0] - a[0], b[1] - a[1]);
      const v = t1 ?? unit(b[0] - a[0], b[1] - a[1]);
      curves = [[a, [a[0] + u[0] * k, a[1] + u[1] * k], [b[0] - v[0] * k, b[1] - v[1] * k], b]];
    }
    let cur = nodes[nodes.length - 1];
    for (const c of curves.length ? curves : [null]) {
      if (c) {
        cur.out = [c[1][0], c[1][1]];
        cur = { x: c[3][0], y: c[3][1], in: [c[2][0], c[2][1]] };
      } else cur = { x: pts[i1][0], y: pts[i1][1] };
      nodes.push(cur);
    }
  }
  // Back at the first split: its incoming handle moves there.
  const last = nodes.pop()!;
  if (last.in) nodes[0].in = last.in;
  return nodes.length >= 2 ? { closed: true, nodes: nodes.map(roundNode) } : null;
}

/** The whole trace: mask, outlines, nesting, simplification and curves. */
export function traceBitmap(img: Bitmap, opts: Partial<TraceOptions> = {}): TraceResult {
  const o = { ...TRACE_DEFAULTS, ...opts };
  const mask = inkMask(img, o.threshold, o.invert);
  const { shapes, removed } = nestRings(traceContours(mask, img.width, img.height), o.speckle);
  const out: TracedShape[] = [];
  let nodes = 0;
  let holes = 0;
  for (const s of shapes) {
    const outer = fitTracedRing(s.outer, o);
    if (!outer) continue;
    const hs = s.holes.map((h) => fitTracedRing(h, o)).filter((h): h is SubPath => !!h);
    nodes += outer.nodes.length + hs.reduce((t, h) => t + h.nodes.length, 0);
    holes += hs.length;
    out.push({ outer, holes: hs });
  }
  return { shapes: out, nodes, holes, removed };
}
