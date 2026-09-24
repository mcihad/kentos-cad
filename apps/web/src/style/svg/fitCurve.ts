import type { Cubic } from './bezier';
import { bez, bezDeriv } from './bezier';
import type { PathNode, Pt, SubPath } from './pathData';

/**
 * Fits cubic Béziers through a polyline within a tolerance (Schneider,
 * "An Algorithm for Automatically Fitting Digitized Curves", Graphics Gems
 * 1990): chord-length parameters, least-squares handles along fixed end
 * tangents, Newton re-parameterisation, and a split at the worst point
 * when one curve is not enough. Straight runs stay lines and sharp turns
 * stay corners. Simplify, stroke to path and offsets use it. Pure.
 */

type V = [number, number];

const sub = (a: Pt, b: Pt): V => [a[0] - b[0], a[1] - b[1]];
const add = (a: Pt, b: Pt): V => [a[0] + b[0], a[1] + b[1]];
const mul = (a: Pt, k: number): V => [a[0] * k, a[1] * k];
const dot = (a: Pt, b: Pt) => a[0] * b[0] + a[1] * b[1];
const len = (a: Pt) => Math.hypot(a[0], a[1]);
const unit = (a: Pt): V => {
  const l = len(a);
  return l > 1e-15 ? [a[0] / l, a[1] / l] : [0, 0];
};

/** Distance from p to the segment a–b. */
export function distToSegment(p: Pt, a: Pt, b: Pt): number {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const l2 = dx * dx + dy * dy;
  const t = l2 > 0 ? Math.max(0, Math.min(1, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2)) : 0;
  return Math.hypot(p[0] - a[0] - dx * t, p[1] - a[1] - dy * t);
}

function chordParams(d: readonly Pt[], first: number, last: number): number[] {
  const u = [0];
  for (let i = first + 1; i <= last; i++) u.push(u[u.length - 1] + len(sub(d[i], d[i - 1])));
  const total = u[u.length - 1] || 1;
  return u.map((v) => v / total);
}

function generate(d: readonly Pt[], first: number, last: number, u: number[], t1: Pt, t2: Pt): Cubic {
  const p0 = d[first];
  const p3 = d[last];
  let c00 = 0;
  let c01 = 0;
  let c11 = 0;
  let x0 = 0;
  let x1 = 0;
  for (let i = 0; i < u.length; i++) {
    const t = u[i];
    const s = 1 - t;
    const b0 = s * s * s;
    const b1 = 3 * t * s * s;
    const b2 = 3 * t * t * s;
    const b3 = t * t * t;
    const a1 = mul(t1, b1);
    const a2 = mul(t2, b2);
    c00 += dot(a1, a1);
    c01 += dot(a1, a2);
    c11 += dot(a2, a2);
    const tmp = sub(d[first + i], add(mul(p0, b0 + b1), mul(p3, b2 + b3)));
    x0 += dot(a1, tmp);
    x1 += dot(a2, tmp);
  }
  const det = c00 * c11 - c01 * c01;
  let alpha1 = det !== 0 ? (x0 * c11 - x1 * c01) / det : 0;
  let alpha2 = det !== 0 ? (c00 * x1 - c01 * x0) / det : 0;
  const segLen = len(sub(p3, p0));
  const eps = 1e-6 * segLen;
  if (alpha1 < eps || alpha2 < eps) {
    // Fall back to the Wu/Barsky heuristic: a third of the chord.
    alpha1 = alpha2 = segLen / 3;
  }
  return [p0, add(p0, mul(t1, alpha1)), add(p3, mul(t2, alpha2)), p3];
}

function maxError(d: readonly Pt[], first: number, last: number, c: Cubic, u: number[]): { err: number; at: number } {
  let err = 0;
  let at = Math.floor((first + last) / 2);
  for (let i = first + 1; i < last; i++) {
    const p = bez(c, u[i - first]);
    const e = (p[0] - d[i][0]) ** 2 + (p[1] - d[i][1]) ** 2;
    if (e >= err) {
      err = e;
      at = i;
    }
  }
  return { err, at };
}

function reparameterize(d: readonly Pt[], first: number, c: Cubic, u: number[]): number[] {
  return u.map((t, i) => {
    const p = d[first + i];
    const q = bez(c, t);
    const q1 = bezDeriv(c, t);
    const s = 1 - t;
    const q2: V = [6 * s * (c[2][0] - 2 * c[1][0] + c[0][0]) + 6 * t * (c[3][0] - 2 * c[2][0] + c[1][0]), 6 * s * (c[2][1] - 2 * c[1][1] + c[0][1]) + 6 * t * (c[3][1] - 2 * c[2][1] + c[1][1])];
    const num = (q[0] - p[0]) * q1[0] + (q[1] - p[1]) * q1[1];
    const den = q1[0] * q1[0] + q1[1] * q1[1] + (q[0] - p[0]) * q2[0] + (q[1] - p[1]) * q2[1];
    if (Math.abs(den) < 1e-18) return t;
    return Math.min(1, Math.max(0, t - num / den));
  });
}

function fitRange(d: readonly Pt[], first: number, last: number, t1: Pt, t2: Pt, tol2: number, out: Cubic[], depth: number): void {
  if (last - first === 1 || depth > 40) {
    const dist = len(sub(d[last], d[first])) / 3;
    out.push([d[first], add(d[first], mul(t1, dist)), add(d[last], mul(t2, dist)), d[last]]);
    return;
  }
  let u = chordParams(d, first, last);
  let c = generate(d, first, last, u, t1, t2);
  let { err, at } = maxError(d, first, last, c, u);
  if (err < tol2) return void out.push(c);
  if (err < tol2 * 16) {
    for (let it = 0; it < 12; it++) {
      u = reparameterize(d, first, c, u);
      c = generate(d, first, last, u, t1, t2);
      ({ err, at } = maxError(d, first, last, c, u));
      if (err < tol2) return void out.push(c);
    }
  }
  // Far off (a long smooth run): halve it, which keeps the pieces even;
  // close: split at the worst point, where the detail is.
  if (err > tol2 * 256) at = Math.floor((first + last) / 2);
  if (at <= first) at = first + 1;
  if (at >= last) at = last - 1;
  const centre = unit(sub(d[at - 1], d[at + 1]));
  const tc: V = len(centre) > 0 ? centre : unit(sub(d[at - 1], d[at]));
  fitRange(d, first, at, t1, tc, tol2, out, depth + 1);
  fitRange(d, at, last, [-tc[0], -tc[1]], t2, tol2, out, depth + 1);
}

/** One run of points as cubics (lines when the run is straight within `tol`). */
export function fitRun(pts: readonly Pt[], tol: number, t1?: Pt, t2?: Pt): (Cubic | null)[] {
  const d = dedupe(pts);
  if (d.length < 2) return [];
  const a = d[0];
  const b = d[d.length - 1];
  if (d.every((p) => distToSegment(p, a, b) <= tol)) return [null];
  const out: Cubic[] = [];
  const s1 = t1 ?? unit(sub(d[1], d[0]));
  const s2 = t2 ?? unit(sub(d[d.length - 2], d[d.length - 1]));
  fitRange(d, 0, d.length - 1, s1, s2, tol * tol, out, 0);
  return out;
}

/** The one cubic that best fits the points with these end tangents (node deletion keeping the shape). */
export function fitOne(pts: readonly Pt[], t1: Pt, t2: Pt): Cubic | null {
  const d = dedupe(pts);
  if (d.length < 2) return null;
  const last = d.length - 1;
  let u = chordParams(d, 0, last);
  let c = generate(d, 0, last, u, t1, t2);
  for (let it = 0; it < 8; it++) {
    u = reparameterize(d, 0, c, u);
    c = generate(d, 0, last, u, t1, t2);
  }
  return c;
}

function dedupe(pts: readonly Pt[]): Pt[] {
  const out: Pt[] = [];
  for (const p of pts) {
    const q = out[out.length - 1];
    if (!q || Math.hypot(p[0] - q[0], p[1] - q[1]) > 1e-12) out.push(p);
  }
  return out;
}

/** Indices of points where the polyline turns by more than `deg` (ends of an open line always). */
export function cornersOf(pts: readonly Pt[], closed: boolean, deg: number): number[] {
  const n = pts.length;
  const limit = Math.cos((deg * Math.PI) / 180);
  const out: number[] = [];
  for (let i = 0; i < n; i++) {
    if (!closed && (i === 0 || i === n - 1)) {
      out.push(i);
      continue;
    }
    const a = pts[(i - 1 + n) % n];
    const b = pts[i];
    const c = pts[(i + 1) % n];
    const u = unit(sub(b, a));
    const v = unit(sub(c, b));
    if (dot(u, v) < limit) out.push(i);
  }
  return out;
}

export interface FitOptions {
  closed: boolean;
  /** Points that must stay corners (nodes of the source), besides sharp turns. */
  corners?: readonly number[];
  /** Turns sharper than this (degrees) are corners; default 30°. */
  cornerDeg?: number;
}

/**
 * A polyline as a sub-path of fitted curves: split at corners, each run
 * fitted within `tol`. A closed polyline without corners is fitted as one
 * smooth loop (the seam's tangents match).
 */
export function fitPolyline(input: readonly Pt[], tol: number, opts: FitOptions): SubPath {
  // Repeated points go; the caller's corner indices follow the kept ones.
  const pts: Pt[] = [];
  const keptAt: number[] = [];
  input.forEach((p, i) => {
    const q = pts[pts.length - 1];
    if (!q || Math.hypot(p[0] - q[0], p[1] - q[1]) > 1e-12) pts.push(p);
    keptAt[i] = pts.length - 1;
  });
  if (opts.closed && pts.length > 2) {
    const a = pts[0];
    const b = pts[pts.length - 1];
    if (Math.hypot(a[0] - b[0], a[1] - b[1]) <= 1e-12) {
      pts.pop();
      for (let i = 0; i < keptAt.length; i++) if (keptAt[i] === pts.length) keptAt[i] = 0;
    }
  }
  const n = pts.length;
  if (n < 2) return { closed: opts.closed, nodes: pts.map((p) => ({ x: p[0], y: p[1] })) };
  const set = new Set([...(opts.corners ?? []).map((i) => keptAt[i]), ...cornersOf(pts, opts.closed, opts.cornerDeg ?? 30)]);
  const corners = [...set].filter((i) => i >= 0 && i < n).sort((a, b) => a - b);
  const pieces: { to: number; curves: (Cubic | null)[] }[] = [];
  const at = (i: number) => pts[((i % n) + n) % n];
  let first = 0;
  if (opts.closed && !corners.length) {
    // A smooth loop: start at 0 with the tangent across the seam.
    const t = unit(sub(at(1), at(-1)));
    pieces.push({ to: n, curves: fitRun([...pts, pts[0]], tol, t, [-t[0], -t[1]]) });
  } else {
    const cs = opts.closed ? corners : corners.includes(0) ? corners : [0, ...corners];
    first = cs[0];
    const count = opts.closed ? cs.length : cs.length - 1;
    for (let k = 0; k < count; k++) {
      const i0 = cs[k];
      const i1 = opts.closed && k === cs.length - 1 ? cs[0] + n : cs[k + 1];
      const run: Pt[] = [];
      for (let i = i0; i <= i1; i++) run.push(at(i));
      pieces.push({ to: i1, curves: fitRun(run, tol) });
    }
  }
  const p0 = at(first);
  let cur: PathNode = { x: p0[0], y: p0[1] };
  const nodes: PathNode[] = [cur];
  for (const piece of pieces) {
    for (const c of piece.curves) {
      if (c) {
        cur.out = [c[1][0], c[1][1]];
        cur = { x: c[3][0], y: c[3][1], in: [c[2][0], c[2][1]] };
      } else {
        const p = at(piece.to);
        cur = { x: p[0], y: p[1] };
      }
      nodes.push(cur);
    }
  }
  if (opts.closed && nodes.length > 1) {
    // The walk came back to the first node: its incoming handle moves there.
    const last = nodes.pop()!;
    if (last.in) nodes[0].in = last.in;
  }
  return { closed: opts.closed, nodes };
}
