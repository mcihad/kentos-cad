import type { Vec2 } from '../../model/geometry';
import { bulgeArc } from '../../model/geom/bulge';
import type { Edge } from '../../model/geom/intersect';
import { faceRings, overlay, type Area, type Ring, type Source } from '../../model/geom/overlay';
import { areaSource, insideArea } from '../../model/geom/region';
import { bez, flattenCubic, segmentCount, segmentCubic, segmentIsLine, subCubic, windingOf, ringSignedArea, type Cubic } from './bezier';
import { distToSegment, fitPolyline } from './fitCurve';
import type { PathNode, Pt, SubPath } from './pathData';

/**
 * Path booleans on the app's plane overlay engine (model/geom/overlay.ts).
 *
 * Curves are flattened to chords within a tolerance (a ten-thousandth of
 * the drawing's size) and every chord remembers the input segment and the
 * parameter range it came from. The overlay keeps input vertices bit for
 * bit, so each edge of a result ring lies on one input chord; runs of
 * edges on the same input segment are turned back into the exact part of
 * that Bézier (de Casteljau). **Curves are kept**: only the new corners
 * where outlines cross move to the crossing, within the tolerance. Edges
 * that no input owns (offsets, stroke outlines) are fitted with cubics,
 * and arcs of round joins become cubics.
 *
 * Fill rules: a nonzero shape is one overlay source as drawn (the overlay's
 * inside test is nonzero winding). An even-odd shape whose rings do not
 * cross is re-oriented by nesting depth; one whose rings cross is split
 * into the faces of its line work and the faces with odd winding kept.
 */

export type FillRule = 'nonzero' | 'evenodd';

export interface RegionInput {
  subs: readonly SubPath[];
  fillRule: FillRule;
}

const v = (p: Pt): Vec2 => ({ x: p[0], y: p[1] });
const segEdge = (a: Pt, b: Pt): Edge => ({ kind: 'seg', a: v(a), b: v(b) });

interface Owner {
  c: Cubic;
  line: boolean;
}

interface Chord {
  a: Pt;
  b: Pt;
  t0: number;
  t1: number;
  owner: number;
}

/** Hit of a result edge on an input chord: the input segment and the parameters of the edge's ends. */
interface Own {
  o: number;
  ta: number;
  tb: number;
}

/**
 * Flattens input sub-paths, remembering where each chord came from, and
 * finds a result edge's chord back (a grid of cells over the chords).
 */
export class Tracer {
  readonly tol: number;
  private readonly cell: number;
  private readonly owners: Owner[] = [];
  private readonly chords: Chord[] = [];
  private readonly grid = new Map<string, number[]>();
  /** Points flattened inside curves (not nodes), by exact coordinates. */
  private readonly samples = new Set<string>();

  constructor(tol: number, extent: number) {
    this.tol = tol;
    this.cell = Math.max(extent / 48, tol * 4, 1e-9);
  }

  private addChord(a: Pt, b: Pt, t0: number, t1: number, owner: number): void {
    const id = this.chords.length;
    this.chords.push({ a, b, t0, t1, owner });
    const s = this.cell;
    const x0 = Math.floor(Math.min(a[0], b[0]) / s);
    const x1 = Math.floor(Math.max(a[0], b[0]) / s);
    const y0 = Math.floor(Math.min(a[1], b[1]) / s);
    const y1 = Math.floor(Math.max(a[1], b[1]) / s);
    for (let x = x0; x <= x1; x++)
      for (let y = y0; y <= y1; y++) {
        const k = `${x},${y}`;
        const list = this.grid.get(k);
        if (list) list.push(id);
        else this.grid.set(k, [id]);
      }
  }

  private addSegment(c: Cubic, line: boolean, out: Pt[]): void {
    const owner = this.owners.length;
    this.owners.push({ c, line });
    if (line) {
      this.addChord(c[0], c[3], 0, 1, owner);
      out.push(c[3]);
      return;
    }
    const { pts, ts } = flattenCubic(c, this.tol);
    for (let k = 1; k < pts.length; k++) {
      this.addChord(pts[k - 1], pts[k], ts[k - 1], ts[k], owner);
      out.push(pts[k]);
      if (k < pts.length - 1) this.samples.add(`${pts[k][0]},${pts[k][1]}`);
    }
  }

  /** Whether p is a point flattened inside a curve (the overlay keeps such points exactly). */
  isSample(p: Pt): boolean {
    return this.samples.has(`${p[0]},${p[1]}`);
  }

  /**
   * A sub-path as a closed ring (an open one is closed by a straight chord,
   * as SVG fills it). The ring does not repeat its first point.
   */
  ring(sp: SubPath): Pt[] {
    const out: Pt[] = [];
    if (!sp.nodes.length) return out;
    const first: Pt = [sp.nodes[0].x, sp.nodes[0].y];
    out.push(first);
    const n = segmentCount(sp);
    for (let i = 0; i < n; i++) this.addSegment(segmentCubic(sp, i), segmentIsLine(sp, i), out);
    if (!sp.closed && out.length > 1) {
      const last = out[out.length - 1];
      if (last[0] !== first[0] || last[1] !== first[1]) this.addSegment([last, last, first, first], true, out);
    }
    const last = out[out.length - 1];
    if (out.length > 1 && last[0] === first[0] && last[1] === first[1]) out.pop();
    return out;
  }

  /** A sub-path as a polyline of cut lines (a closed one ends where it starts). */
  line(sp: SubPath): Pt[] {
    const out: Pt[] = [];
    if (!sp.nodes.length) return out;
    out.push([sp.nodes[0].x, sp.nodes[0].y]);
    const n = segmentCount(sp);
    for (let i = 0; i < n; i++) this.addSegment(segmentCubic(sp, i), segmentIsLine(sp, i), out);
    return out;
  }

  /** The input chord a result edge p→q lies on, with the parameters of p and q. */
  find(p: Pt, q: Pt): Own | null {
    const m: Pt = [(p[0] + q[0]) / 2, (p[1] + q[1]) / 2];
    // Overlay vertices merge within 1e-6; a little more covers the rounding.
    const eps = 4e-6 + this.tol * 1e-6;
    const s = this.cell;
    const ids = new Set<number>();
    // The cells around the midpoint (a chord on a cell border may be filed on either side).
    for (let x = Math.floor((m[0] - eps) / s); x <= Math.floor((m[0] + eps) / s); x++)
      for (let y = Math.floor((m[1] - eps) / s); y <= Math.floor((m[1] + eps) / s); y++) for (const id of this.grid.get(`${x},${y}`) ?? []) ids.add(id);
    let best: Own | null = null;
    let bestD = eps;
    for (const id of ids) {
      const c = this.chords[id];
      const d = Math.max(distToSegment(p, c.a, c.b), distToSegment(q, c.a, c.b), distToSegment(m, c.a, c.b));
      if (d > bestD) continue;
      bestD = d;
      const at = (r: Pt) => {
        const dx = c.b[0] - c.a[0];
        const dy = c.b[1] - c.a[1];
        const l2 = dx * dx + dy * dy;
        const u = l2 > 0 ? Math.max(0, Math.min(1, ((r[0] - c.a[0]) * dx + (r[1] - c.a[1]) * dy) / l2)) : 0;
        return c.t0 + (c.t1 - c.t0) * u;
      };
      best = { o: c.owner, ta: at(p), tb: at(q) };
    }
    return best;
  }

  owner(o: number): Owner {
    return this.owners[o];
  }
}

// ── Regions ────────────────────────────────────────────────────────────

const ringEdges = (ring: readonly Pt[]): Edge[] => ring.map((p, i) => segEdge(p, ring[(i + 1) % ring.length]));

/** Whether any two chords of the rings cross or touch (neighbours on one ring excepted). */
export function ringsCross(rings: readonly (readonly Pt[])[]): boolean {
  const segs: { a: Pt; b: Pt; r: number; i: number; n: number; minX: number; maxX: number; minY: number; maxY: number }[] = [];
  rings.forEach((ring, r) =>
    ring.forEach((a, i) => {
      const b = ring[(i + 1) % ring.length];
      segs.push({ a, b, r, i, n: ring.length, minX: Math.min(a[0], b[0]), maxX: Math.max(a[0], b[0]), minY: Math.min(a[1], b[1]), maxY: Math.max(a[1], b[1]) });
    }),
  );
  segs.sort((s, t) => s.minX - t.minX);
  const eps = 1e-9;
  for (let x = 0; x < segs.length; x++) {
    const s = segs[x];
    for (let y = x + 1; y < segs.length; y++) {
      const t = segs[y];
      if (t.minX > s.maxX + eps) break;
      if (t.minY > s.maxY + eps || t.maxY < s.minY - eps) continue;
      if (s.r === t.r && (Math.abs(s.i - t.i) === 1 || Math.abs(s.i - t.i) === s.n - 1)) continue;
      if (segmentsMeet(s.a, s.b, t.a, t.b, eps)) return true;
    }
  }
  return false;
}

function segmentsMeet(a: Pt, b: Pt, c: Pt, d: Pt, eps: number): boolean {
  const o = (p: Pt, q: Pt, r: Pt) => (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
  const d1 = o(c, d, a);
  const d2 = o(c, d, b);
  const d3 = o(a, b, c);
  const d4 = o(a, b, d);
  if (((d1 > eps && d2 < -eps) || (d1 < -eps && d2 > eps)) && ((d3 > eps && d4 < -eps) || (d3 < -eps && d4 > eps))) return true;
  // Touching or collinear overlaps.
  return distToSegment(a, c, d) <= eps || distToSegment(b, c, d) <= eps || distToSegment(c, a, b) <= eps || distToSegment(d, a, b) <= eps;
}

/** The fill of a shape as one overlay source (nonzero-clean), its rings flattened through the tracer. */
export function regionSource(input: RegionInput, tr: Tracer): Source {
  const rings = input.subs.map((sp) => tr.ring(sp)).filter((r) => r.length >= 2);
  const points = rings.flat().map(v);
  if (input.fillRule === 'nonzero') return { edges: rings.flatMap(ringEdges), points };
  if (!ringsCross(rings)) {
    // Nested rings alternate: even depth one way round, odd depth the other.
    const oriented = rings.map((ring, i) => {
      const depth = rings.reduce((d, other, j) => (j !== i && windingOf(other, ring[0]) !== 0 ? d + 1 : d), 0);
      const pos = ringSignedArea(ring) > 0;
      return pos === (depth % 2 === 0) ? ring : [...ring].reverse();
    });
    return { edges: oriented.flatMap(ringEdges), points };
  }
  // Crossing rings: faces of the line work, kept where the winding is odd.
  const rs = faceRings([{ edges: rings.flatMap(ringEdges), points, cut: true }]);
  const inBox = (r: (typeof rs)[number], p: Vec2) => p.x >= r.box.minX && p.x <= r.box.maxX && p.y >= r.box.minY && p.y <= r.box.maxY;
  const faces = rs.filter((r) => r.area > 0).sort((a, b) => a.area - b.area);
  const groups = rs.filter((r) => r.area < 0);
  const host = new Map(groups.map((g) => [g, faces.find((f) => inBox(f, g.probe) && insideArea({ outer: f.ring, holes: [] }, g.probe))]));
  const kept: Area[] = [];
  for (const f of faces) {
    const p: Pt = [f.probe.x, f.probe.y];
    const w = rings.reduce((s, r) => s + windingOf(r, p), 0);
    if (Math.abs(w) % 2 === 1) kept.push({ outer: f.ring, holes: groups.filter((g) => host.get(g) === f).map((g) => g.ring) });
  }
  return areaSource(kept);
}

// ── Back to sub-paths ──────────────────────────────────────────────────

interface REdge {
  p: Pt;
  q: Pt;
  bulge: number;
  own: Own | null;
}

const follows = (a: REdge, b: REdge) => {
  if (a.bulge || b.bulge) return false;
  if (!a.own || !b.own) return !a.own && !b.own;
  if (a.own.o !== b.own.o || Math.abs(a.own.tb - b.own.ta) > 1e-6) return false;
  const da = a.own.tb - a.own.ta;
  const db = b.own.tb - b.own.ta;
  return da === 0 || db === 0 || da > 0 === db > 0;
};

/** Cubic pieces of a circular arc edge (bulge form) from p to q. */
function arcCubics(p: Pt, q: Pt, bulge: number): Cubic[] {
  const arc = bulgeArc(v(p), v(q), bulge);
  if (!arc) return [[p, p, q, q]];
  const n = Math.max(1, Math.ceil(Math.abs(arc.sweep) / (Math.PI / 2) - 1e-9));
  const step = arc.sweep / n;
  const k = (4 / 3) * Math.tan(step / 4);
  const at = (a: number): Pt => [arc.c.x + arc.r * Math.cos(a), arc.c.y + arc.r * Math.sin(a)];
  const tan = (a: number): Pt => [-arc.r * Math.sin(a), arc.r * Math.cos(a)];
  const out: Cubic[] = [];
  for (let s = 0; s < n; s++) {
    const a0 = arc.a0 + s * step;
    const a1 = a0 + step;
    const p0 = s === 0 ? p : at(a0);
    const p3 = s === n - 1 ? q : at(a1);
    const t0 = tan(a0);
    const t1 = tan(a1);
    out.push([p0, [p0[0] + k * t0[0], p0[1] + k * t0[1]], [p3[0] - k * t1[0], p3[1] - k * t1[1]], p3]);
  }
  return out;
}

class Builder {
  readonly nodes: PathNode[] = [];

  start(p: Pt): void {
    this.nodes.push({ x: p[0], y: p[1] });
  }

  private get last(): PathNode {
    return this.nodes[this.nodes.length - 1];
  }

  lineTo(p: Pt): void {
    const l = this.last;
    if (Math.hypot(l.x - p[0], l.y - p[1]) < 1e-12) return;
    this.nodes.push({ x: p[0], y: p[1] });
  }

  curveTo(c1: Pt, c2: Pt, p: Pt): void {
    const l = this.last;
    if (Math.hypot(l.x - p[0], l.y - p[1]) < 1e-12 && Math.hypot(c1[0] - p[0], c1[1] - p[1]) < 1e-12) return;
    l.out = [c1[0], c1[1]];
    this.nodes.push({ x: p[0], y: p[1], in: [c2[0], c2[1]] });
  }

  /** Appends a fitted sub-path whose first node is the current end. */
  append(sp: SubPath): void {
    const [first, ...rest] = sp.nodes;
    if (first?.out) this.last.out = first.out;
    for (const n of rest) this.nodes.push({ ...n });
  }

  close(): SubPath {
    const nodes = this.nodes;
    if (nodes.length > 1) {
      const a = nodes[0];
      const b = nodes[nodes.length - 1];
      if (Math.hypot(a.x - b.x, a.y - b.y) < 1e-9) {
        nodes.pop();
        if (b.in) a.in = b.in;
        else delete a.in;
      }
    }
    return { closed: true, nodes };
  }
}

/**
 * A result ring back as a sub-path: runs on one input segment become that
 * segment's exact part, arcs become cubics, everything else is fitted
 * within `fitTol` (0 keeps it as lines).
 */
export function ringToSubPath(ring: Ring, tr: Tracer | null, fitTol: number): SubPath {
  const pts: Pt[] = ring.pts.map((p) => [p.x, p.y]);
  const n = pts.length;
  const edges: REdge[] = pts.map((p, i) => {
    const q = pts[(i + 1) % n];
    const bulge = ring.bulges?.[i] ?? 0;
    return { p, q, bulge, own: !bulge && tr ? tr.find(p, q) : null };
  });
  if (n < 2) return { closed: true, nodes: pts.map((p) => ({ x: p[0], y: p[1] })) };
  // A ring that is one free run all round is fitted as a smooth loop.
  if (edges.every((e) => !e.own && !e.bulge)) {
    return fitTol > 0 ? fitPolyline(pts, fitTol, { closed: true, cornerDeg: 35 }) : { closed: true, nodes: pts.map((p) => ({ x: p[0], y: p[1] })) };
  }
  let start = edges.findIndex((e, i) => !follows(edges[(i - 1 + n) % n], e));
  // One input segment all round (a single-node loop): break it in two.
  const forced = start < 0;
  if (forced) start = 0;
  const runs: REdge[][] = [];
  for (let k = 0; k < n; k++) {
    const e = edges[(start + k) % n];
    const cur = runs[runs.length - 1];
    if (cur && follows(cur[cur.length - 1], e) && !(forced && k === Math.floor(n / 2))) cur.push(e);
    else runs.push([e]);
  }
  const b = new Builder();
  b.start(runs[0][0].p);
  for (const run of runs) {
    const first = run[0];
    const end = run[run.length - 1].q;
    if (first.bulge) {
      for (const c of arcCubics(first.p, first.q, first.bulge)) b.curveTo(c[1], c[2], c[3]);
    } else if (first.own && tr) {
      const o = tr.owner(first.own.o);
      if (o.line) b.lineTo(end);
      else {
        const s = subCubic(o.c, first.own.ta, run[run.length - 1].own!.tb);
        // The crossing may sit a hair off the curve: carry the handles with the ends.
        const p0 = first.p;
        b.curveTo([s[1][0] + p0[0] - s[0][0], s[1][1] + p0[1] - s[0][1]], [s[2][0] + end[0] - s[3][0], s[2][1] + end[1] - s[3][1]], end);
      }
    } else if (fitTol > 0 && run.length > 1) {
      b.append(fitPolyline([first.p, ...run.map((e) => e.q)], fitTol, { closed: false, cornerDeg: 35 }));
    } else for (const e of run) b.lineTo(e.q);
  }
  return b.close();
}

export const areasToSubPaths = (areas: readonly Area[], tr: Tracer | null, fitTol: number): SubPath[] =>
  areas.flatMap((a) => [a.outer, ...a.holes].map((r) => ringToSubPath(r, tr, fitTol))).filter((sp) => sp.nodes.length >= 2);

// ── Booleans ───────────────────────────────────────────────────────────

export type BoolOp = 'union' | 'difference' | 'intersection' | 'exclusion' | 'division';

/** Size of everything involved (for tolerances). */
export function extentOf(list: readonly (readonly SubPath[])[]): number {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const subs of list)
    for (const sp of subs)
      for (const n of sp.nodes)
        for (const p of [[n.x, n.y], n.in, n.out]) {
          if (!p) continue;
          minX = Math.min(minX, p[0]);
          minY = Math.min(minY, p[1]);
          maxX = Math.max(maxX, p[0]);
          maxY = Math.max(maxY, p[1]);
        }
  return Number.isFinite(minX) ? Math.max(maxX - minX, maxY - minY, 1e-6) : 1;
}

/** Flattening tolerance for a drawing of this size. */
export const tolFor = (extent: number) => Math.max(extent * 1e-4, 1e-7);

/**
 * A boolean of fill regions, bottom first. Union, intersection and
 * exclusion (odd count) use all inputs; difference takes the others away
 * from the first; division cuts the first along the others' outlines.
 * Every result is a list of sub-paths (division gives one per piece).
 */
export function booleanOp(op: BoolOp, inputs: readonly RegionInput[]): SubPath[][] {
  if (!inputs.length) return [];
  const extent = extentOf(inputs.map((i) => i.subs));
  const tr = new Tracer(tolFor(extent), extent);
  if (op === 'division') {
    const [bottom, ...rest] = inputs;
    const src = regionSource(bottom, tr);
    const cuts: Edge[] = [];
    const points: Vec2[] = [];
    for (const r of rest)
      for (const sp of r.subs) {
        const line = tr.line(sp);
        for (let i = 1; i < line.length; i++) cuts.push(segEdge(line[i - 1], line[i]));
        points.push(...line.map(v));
      }
    const pieces = overlay([src, { edges: cuts, points, cut: true }], 'first');
    return pieces.map((a) => areasToSubPaths([a], tr, 0));
  }
  const sources = inputs.map((i) => regionSource(i, tr));
  const rule = op === 'union' ? 'any' : op === 'intersection' ? 'all' : op === 'exclusion' ? 'odd' : 'firstNotOthers';
  if (op === 'intersection' && sources.length < 2) return [];
  return [areasToSubPaths(overlay(sources, rule), tr, 0)];
}

// ── Cut path ───────────────────────────────────────────────────────────

/**
 * The bottom path's outline cut where the others' outlines cross it: open
 * pieces, each exactly the part of the curves it covers (Béziers split at
 * the crossings). A sub-path nothing crosses stays as it is.
 */
export function cutPath(target: readonly SubPath[], cutters: readonly (readonly SubPath[])[]): SubPath[] {
  const extent = extentOf([target, ...cutters]);
  const tol = tolFor(extent);
  const tr = new Tracer(tol, extent);
  const knives: { a: Pt; b: Pt; minX: number; maxX: number; minY: number; maxY: number }[] = [];
  for (const subs of cutters)
    for (const sp of subs) {
      const line = sp.closed ? [...tr.ring(sp), [sp.nodes[0].x, sp.nodes[0].y] as Pt] : tr.line(sp);
      for (let i = 1; i < line.length; i++) {
        const a = line[i - 1];
        const b = line[i];
        knives.push({ a, b, minX: Math.min(a[0], b[0]), maxX: Math.max(a[0], b[0]), minY: Math.min(a[1], b[1]), maxY: Math.max(a[1], b[1]) });
      }
    }
  const out: SubPath[] = [];
  for (const sp of target) {
    const n = segmentCount(sp);
    const cuts: { seg: number; t: number }[] = [];
    for (let i = 0; i < n; i++) {
      const c = segmentCubic(sp, i);
      const line = segmentIsLine(sp, i);
      const { pts, ts } = line ? { pts: [c[0], c[3]], ts: [0, 1] } : flattenCubic(c, tol);
      for (let k = 1; k < pts.length; k++) {
        const a = pts[k - 1];
        const b = pts[k];
        for (const kn of knives) {
          if (kn.minX > Math.max(a[0], b[0]) || kn.maxX < Math.min(a[0], b[0]) || kn.minY > Math.max(a[1], b[1]) || kn.maxY < Math.min(a[1], b[1])) continue;
          const u = crossParam(a, b, kn.a, kn.b);
          if (u === null) continue;
          let t = ts[k - 1] + (ts[k] - ts[k - 1]) * u;
          if (!line) t = refineOnLine(c, t, kn.a, kn.b);
          cuts.push({ seg: i, t });
        }
      }
    }
    out.push(...splitAt(sp, cuts));
  }
  return out;
}

/** Parameter along a→b where it crosses c→d (ends included), or null. */
function crossParam(a: Pt, b: Pt, c: Pt, d: Pt): number | null {
  const rx = b[0] - a[0];
  const ry = b[1] - a[1];
  const sx = d[0] - c[0];
  const sy = d[1] - c[1];
  const den = rx * sy - ry * sx;
  if (Math.abs(den) < 1e-18) return null;
  const t = ((c[0] - a[0]) * sy - (c[1] - a[1]) * sx) / den;
  const u = ((c[0] - a[0]) * ry - (c[1] - a[1]) * rx) / den;
  const e = 1e-12;
  return t >= -e && t <= 1 + e && u >= -e && u <= 1 + e ? Math.min(1, Math.max(0, t)) : null;
}

/** Newton steps moving t onto the line c–d (the flattened crossing is within the tolerance already). */
function refineOnLine(cu: Cubic, t0: number, c: Pt, d: Pt): number {
  const nx = -(d[1] - c[1]);
  const ny = d[0] - c[0];
  let t = t0;
  for (let it = 0; it < 6; it++) {
    const p = bez(cu, t);
    const f = (p[0] - c[0]) * nx + (p[1] - c[1]) * ny;
    const h = 1e-6;
    const q = bez(cu, Math.min(1, t + h));
    const df = ((q[0] - c[0]) * nx + (q[1] - c[1]) * ny - f) / h;
    if (Math.abs(df) < 1e-18) break;
    const next = t - f / df;
    if (!(next >= 0 && next <= 1) || Math.abs(next - t0) > 0.05) break;
    t = next;
  }
  return t;
}

/** A sub-path split at (segment, t) points into open pieces. */
export function splitAt(sp: SubPath, cutsIn: readonly { seg: number; t: number }[]): SubPath[] {
  const n = segmentCount(sp);
  // Cuts at a segment's end are cuts at the next one's start; near-equal ones are one.
  const norm = cutsIn
    .map((c) => (c.t >= 1 - 1e-9 ? { seg: (c.seg + 1) % Math.max(n, 1), t: 0 } : c.t <= 1e-9 ? { seg: c.seg, t: 0 } : c))
    .filter((c) => sp.closed || c.seg < n)
    .sort((a, b) => a.seg - b.seg || a.t - b.t);
  const cuts: { seg: number; t: number }[] = [];
  for (const c of norm) {
    const l = cuts[cuts.length - 1];
    if (!l || l.seg !== c.seg || c.t - l.t > 1e-9) cuts.push(c);
  }
  if (!sp.closed) {
    const inner = cuts.filter((c) => !(c.seg === 0 && c.t === 0) && c.seg < n);
    if (!inner.length) return [sp];
    return piecesBetween(sp, [{ seg: 0, t: 0 }, ...inner, { seg: n - 1, t: 1 }]);
  }
  if (!cuts.length) return [sp];
  return piecesBetween(sp, [...cuts, { seg: cuts[0].seg + n, t: cuts[0].t }]);
}

/** Open pieces between consecutive cut points (segment indices may run past the end of a closed ring). */
function piecesBetween(sp: SubPath, marks: readonly { seg: number; t: number }[]): SubPath[] {
  const n = segmentCount(sp);
  const out: SubPath[] = [];
  for (let k = 0; k + 1 < marks.length; k++) {
    const a = marks[k];
    const z = marks[k + 1];
    const b = new Builder();
    let started = false;
    for (let s = a.seg; s <= z.seg; s++) {
      const t0 = s === a.seg ? a.t : 0;
      const t1 = s === z.seg ? z.t : 1;
      if (t1 - t0 <= 1e-12 && !(s === a.seg && s === z.seg)) continue;
      const i = ((s % n) + n) % n;
      const c = segmentCubic(sp, i);
      const part = subCubic(c, t0, t1);
      if (!started) {
        b.start(part[0]);
        started = true;
      }
      if (segmentIsLine(sp, i)) b.lineTo(part[3]);
      else b.curveTo(part[1], part[2], part[3]);
    }
    if (b.nodes.length >= 2) out.push({ closed: false, nodes: b.nodes });
  }
  return out;
}
