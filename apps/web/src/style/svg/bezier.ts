import type { PathNode, Pt, SubPath } from './pathData';

/**
 * Cubic Bézier helpers for the SVG editor's path operations: segments of a
 * sub-path, points and tangents, de Casteljau splitting, flattening to a
 * tolerance, lengths and nearest points. A straight segment is a cubic
 * whose controls sit on its ends, so every segment has one form. Pure.
 */

export type Cubic = readonly [Pt, Pt, Pt, Pt];

/** Segment i of a sub-path runs from node i to node i+1 (the last one of a closed sub-path back to node 0). */
export function segmentCount(sp: SubPath): number {
  const n = sp.nodes.length;
  if (n < 2) return sp.closed && n === 1 && (sp.nodes[0].in || sp.nodes[0].out) ? 1 : 0;
  return sp.closed ? n : n - 1;
}

export const isLineSeg = (a: PathNode, b: PathNode) => !a.out && !b.in;

/** The cubic of segment i (controls on the ends for a straight segment). */
export function segmentCubic(sp: SubPath, i: number): Cubic {
  const a = sp.nodes[i];
  const b = sp.nodes[(i + 1) % sp.nodes.length];
  return [[a.x, a.y], a.out ?? [a.x, a.y], b.in ?? [b.x, b.y], [b.x, b.y]];
}

export const segmentIsLine = (sp: SubPath, i: number) => isLineSeg(sp.nodes[i], sp.nodes[(i + 1) % sp.nodes.length]);

export function bez(c: Cubic, t: number): Pt {
  const u = 1 - t;
  const a = u * u * u;
  const b = 3 * u * u * t;
  const d = 3 * u * t * t;
  const e = t * t * t;
  return [a * c[0][0] + b * c[1][0] + d * c[2][0] + e * c[3][0], a * c[0][1] + b * c[1][1] + d * c[2][1] + e * c[3][1]];
}

/** First derivative at t. */
export function bezDeriv(c: Cubic, t: number): Pt {
  const u = 1 - t;
  const a = 3 * u * u;
  const b = 6 * u * t;
  const d = 3 * t * t;
  return [
    a * (c[1][0] - c[0][0]) + b * (c[2][0] - c[1][0]) + d * (c[3][0] - c[2][0]),
    a * (c[1][1] - c[0][1]) + b * (c[2][1] - c[1][1]) + d * (c[3][1] - c[2][1]),
  ];
}

/**
 * Unit tangent at t. At an end whose control coincides with it the first
 * derivative vanishes; the direction then comes from the next control.
 */
export function bezTangent(c: Cubic, t: number): Pt {
  let d = bezDeriv(c, t);
  if (Math.hypot(d[0], d[1]) < 1e-12) {
    const q = t < 0.5 ? (Math.hypot(c[2][0] - c[0][0], c[2][1] - c[0][1]) > 1e-12 ? c[2] : c[3]) : Math.hypot(c[3][0] - c[1][0], c[3][1] - c[1][1]) > 1e-12 ? c[1] : c[0];
    const p = t < 0.5 ? c[0] : c[3];
    d = t < 0.5 ? [q[0] - p[0], q[1] - p[1]] : [p[0] - q[0], p[1] - q[1]];
  }
  const l = Math.hypot(d[0], d[1]) || 1;
  return [d[0] / l, d[1] / l];
}

const lerp = (a: Pt, b: Pt, t: number): Pt => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];

/** de Casteljau split at t: the part before and the part after. */
export function splitCubic(c: Cubic, t: number): [Cubic, Cubic] {
  const q0 = lerp(c[0], c[1], t);
  const q1 = lerp(c[1], c[2], t);
  const q2 = lerp(c[2], c[3], t);
  const r0 = lerp(q0, q1, t);
  const r1 = lerp(q1, q2, t);
  const s = lerp(r0, r1, t);
  return [
    [c[0], q0, r0, s],
    [s, r1, q2, c[3]],
  ];
}

/** The part of the curve between t0 and t1 (t0 > t1 gives it reversed). */
export function subCubic(c: Cubic, t0: number, t1: number): Cubic {
  if (t0 > t1) return reverseCubic(subCubic(c, t1, t0));
  if (t0 <= 0 && t1 >= 1) return c;
  const right = t0 > 0 ? splitCubic(c, t0)[1] : c;
  if (t1 >= 1) return right;
  const u = (t1 - t0) / (1 - t0);
  return splitCubic(right, u)[0];
}

export const reverseCubic = (c: Cubic): Cubic => [c[3], c[2], c[1], c[0]];

/**
 * Number of equal parameter steps that keep the chords within `tol` of the
 * curve (Wang's bound for cubics), capped for degenerate input.
 */
export function flattenSteps(c: Cubic, tol: number, max = 256): number {
  const m = Math.max(Math.hypot(c[0][0] - 2 * c[1][0] + c[2][0], c[0][1] - 2 * c[1][1] + c[2][1]), Math.hypot(c[1][0] - 2 * c[2][0] + c[3][0], c[1][1] - 2 * c[2][1] + c[3][1]));
  if (!(m > 0) || !(tol > 0)) return 1;
  return Math.max(1, Math.min(max, Math.ceil(Math.sqrt((0.75 * m) / tol))));
}

/** Chord ends along a cubic (both ends included) with their parameters. */
export function flattenCubic(c: Cubic, tol: number): { pts: Pt[]; ts: number[] } {
  const n = flattenSteps(c, tol);
  const pts: Pt[] = [c[0]];
  const ts = [0];
  for (let k = 1; k < n; k++) {
    pts.push(bez(c, k / n));
    ts.push(k / n);
  }
  pts.push(c[3]);
  ts.push(1);
  return { pts, ts };
}

/** A sub-path as a polyline within `tol`; a closed one does not repeat its start (the closing chord is implied). */
export function flattenSubPathTol(sp: SubPath, tol: number): Pt[] {
  const out: Pt[] = [];
  const n = segmentCount(sp);
  if (!sp.nodes.length) return out;
  out.push([sp.nodes[0].x, sp.nodes[0].y]);
  for (let i = 0; i < n; i++) {
    if (segmentIsLine(sp, i)) {
      const b = sp.nodes[(i + 1) % sp.nodes.length];
      out.push([b.x, b.y]);
      continue;
    }
    const { pts } = flattenCubic(segmentCubic(sp, i), tol);
    for (let k = 1; k < pts.length; k++) out.push(pts[k]);
  }
  // A closed sub-path came back to its start: the ring does not repeat it.
  if (sp.closed && out.length > 1) {
    const a = out[0];
    const b = out[out.length - 1];
    if (Math.abs(a[0] - b[0]) < 1e-12 && Math.abs(a[1] - b[1]) < 1e-12) out.pop();
  }
  return out;
}

// Gauss–Legendre nodes on [0, 1] (8 points): exact enough for symbol-sized curves.
const GX = [0.0198550717512319, 0.1016667612931866, 0.2372337950418355, 0.4082826787521751, 0.5917173212478249, 0.7627662049581645, 0.8983332387068134, 0.9801449282487681];
const GW = [0.0506142681451881, 0.1111905172266872, 0.1568533229389436, 0.1813418916891810, 0.1813418916891810, 0.1568533229389436, 0.1111905172266872, 0.0506142681451881];

/** Arc length between t0 and t1 (split in halves for accuracy on tight curves). */
export function cubicLength(c: Cubic, t0 = 0, t1 = 1): number {
  if (c[0][0] === c[1][0] && c[0][1] === c[1][1] && c[2][0] === c[3][0] && c[2][1] === c[3][1]) {
    const a = bez(c, t0);
    const b = bez(c, t1);
    return Math.hypot(b[0] - a[0], b[1] - a[1]);
  }
  let s = 0;
  const parts = 4;
  for (let p = 0; p < parts; p++) {
    const a = t0 + ((t1 - t0) * p) / parts;
    const b = t0 + ((t1 - t0) * (p + 1)) / parts;
    for (let k = 0; k < GX.length; k++) {
      const d = bezDeriv(c, a + (b - a) * GX[k]);
      s += GW[k] * (b - a) * Math.hypot(d[0], d[1]);
    }
  }
  return s;
}

/** Length of segment i of a sub-path. */
export const segmentLength = (sp: SubPath, i: number) => cubicLength(segmentCubic(sp, i));

/** Parameter on the curve nearest to p (sampling, then Newton steps). */
export function nearestOnCubic(c: Cubic, p: Pt): { t: number; p: Pt; d: number } {
  let bestT = 0;
  let bestD = Infinity;
  const n = 32;
  for (let k = 0; k <= n; k++) {
    const q = bez(c, k / n);
    const d = (q[0] - p[0]) ** 2 + (q[1] - p[1]) ** 2;
    if (d < bestD) {
      bestD = d;
      bestT = k / n;
    }
  }
  let t = bestT;
  for (let it = 0; it < 8; it++) {
    const q = bez(c, t);
    const d1 = bezDeriv(c, t);
    // Second derivative for the Newton step on (B(t) − p)·B'(t) = 0.
    const u = 1 - t;
    const d2: Pt = [6 * u * (c[2][0] - 2 * c[1][0] + c[0][0]) + 6 * t * (c[3][0] - 2 * c[2][0] + c[1][0]), 6 * u * (c[2][1] - 2 * c[1][1] + c[0][1]) + 6 * t * (c[3][1] - 2 * c[2][1] + c[1][1])];
    const f = (q[0] - p[0]) * d1[0] + (q[1] - p[1]) * d1[1];
    const df = d1[0] * d1[0] + d1[1] * d1[1] + (q[0] - p[0]) * d2[0] + (q[1] - p[1]) * d2[1];
    if (Math.abs(df) < 1e-18) break;
    const next = Math.min(1, Math.max(0, t - f / df));
    if (Math.abs(next - t) < 1e-12) {
      t = next;
      break;
    }
    t = next;
  }
  const q = bez(c, t);
  const d = Math.hypot(q[0] - p[0], q[1] - p[1]);
  if (d * d > bestD + 1e-18) {
    const b = bez(c, bestT);
    return { t: bestT, p: b, d: Math.sqrt(bestD) };
  }
  return { t, p: q, d };
}

/** The parameter where the curve is `dist` away (straight line) from its start (`fromEnd`: from its end). */
export function paramAtDistance(c: Cubic, dist: number, fromEnd: boolean): number | null {
  const o = fromEnd ? c[3] : c[0];
  const f = (t: number) => Math.hypot(bez(c, t)[0] - o[0], bez(c, t)[1] - o[1]) - dist;
  const n = 64;
  let prevT = fromEnd ? 1 : 0;
  let prevF = f(prevT);
  for (let k = 1; k <= n; k++) {
    const t = fromEnd ? 1 - k / n : k / n;
    const v = f(t);
    if (prevF <= 0 && v >= 0) {
      let lo = prevT;
      let hi = t;
      for (let it = 0; it < 60; it++) {
        const m = (lo + hi) / 2;
        if (f(m) < 0) lo = m;
        else hi = m;
      }
      return (lo + hi) / 2;
    }
    prevT = t;
    prevF = v;
  }
  return null;
}

/** Nodes of a sub-path in the opposite direction (handles swap sides). */
export function reverseSubPath(sp: SubPath): SubPath {
  const nodes = sp.nodes.map((n) => ({ ...n, in: n.out, out: n.in })).reverse();
  for (const n of nodes) {
    if (!n.in) delete n.in;
    if (!n.out) delete n.out;
  }
  if (sp.closed && nodes.length > 1) {
    // Node 0 stays first so a closed ring keeps its start.
    nodes.unshift(nodes.pop()!);
  }
  return { closed: sp.closed, nodes };
}

/** Signed area of a ring (shoelace, y down: positive is clockwise on screen). */
export function ringSignedArea(pts: readonly Pt[]): number {
  let a = 0;
  for (let i = 0; i < pts.length; i++) {
    const p = pts[i];
    const q = pts[(i + 1) % pts.length];
    a += p[0] * q[1] - q[0] * p[1];
  }
  return a / 2;
}

/** Winding number of a closed polyline around p. */
export function windingOf(ring: readonly Pt[], p: Pt): number {
  let w = 0;
  const n = ring.length;
  for (let i = 0; i < n; i++) {
    const a = ring[i];
    const b = ring[(i + 1) % n];
    const cross = (b[0] - a[0]) * (p[1] - a[1]) - (p[0] - a[0]) * (b[1] - a[1]);
    if (a[1] <= p[1]) {
      if (b[1] > p[1] && cross > 0) w++;
    } else if (b[1] <= p[1] && cross < 0) w--;
  }
  return w;
}
