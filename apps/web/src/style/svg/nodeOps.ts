import { bez, bezTangent, paramAtDistance, segmentCount, segmentCubic, segmentIsLine, splitCubic, type Cubic } from './bezier';
import { fitOne } from './fitCurve';
import type { PathNode, Pt, SubPath } from './pathData';

/**
 * Node editing of a path (Inkscape's node tool, and the CAD corner tools):
 * node types (cusp, smooth, symmetric, auto), new nodes in the middle of
 * segments, deleting nodes while keeping the shape, joining and breaking,
 * deleting segments, segments to lines or curves, fillet and chamfer of a
 * corner, aligning and distributing nodes. A node is addressed by its
 * sub-path and index; every function returns new sub-paths and leaves its
 * input alone. Pure.
 */

export interface NodeRef {
  sub: number;
  index: number;
}

export type NodeType = 'cusp' | 'smooth' | 'symmetric' | 'auto';

export const refKey = (r: NodeRef) => `${r.sub}:${r.index}`;

const clone = (subs: readonly SubPath[]): SubPath[] => subs.map((sp) => ({ closed: sp.closed, nodes: sp.nodes.map((n) => ({ ...n })) }));
const len = (x: number, y: number) => Math.hypot(x, y);

/** Neighbours of node i along its sub-path (null past an open end). */
function neighbours(sp: SubPath, i: number): { prev: PathNode | null; next: PathNode | null } {
  const n = sp.nodes.length;
  const prev = i > 0 ? sp.nodes[i - 1] : sp.closed && n > 1 ? sp.nodes[n - 1] : null;
  const next = i < n - 1 ? sp.nodes[i + 1] : sp.closed && n > 1 ? sp.nodes[0] : null;
  return { prev, next };
}

/** The node's type: stored, or read from its handles (in line → smooth, in line and equal → symmetric). */
export function nodeTypeOf(sp: SubPath, i: number): NodeType {
  const node = sp.nodes[i];
  if (node.type) return node.type;
  if (!node.in || !node.out) return 'cusp';
  const a: Pt = [node.x - node.in[0], node.y - node.in[1]];
  const b: Pt = [node.out[0] - node.x, node.out[1] - node.y];
  const la = len(a[0], a[1]);
  const lb = len(b[0], b[1]);
  if (la < 1e-12 || lb < 1e-12) return 'cusp';
  const cross = (a[0] * b[1] - a[1] * b[0]) / (la * lb);
  const dot = (a[0] * b[0] + a[1] * b[1]) / (la * lb);
  if (Math.abs(cross) > 0.01 || dot < 0) return 'cusp';
  return Math.abs(la - lb) < 1e-6 * Math.max(la, lb, 1) ? 'symmetric' : 'smooth';
}

/** Auto-smooth handles: along the neighbours' chord, a third of the way to each. */
export function autoHandles(sp: SubPath, i: number): { in?: Pt; out?: Pt } {
  const node = sp.nodes[i];
  const { prev, next } = neighbours(sp, i);
  if (!prev && !next) return {};
  const ax = prev ? prev.x : node.x;
  const ay = prev ? prev.y : node.y;
  const bx = next ? next.x : node.x;
  const by = next ? next.y : node.y;
  const d = len(bx - ax, by - ay);
  if (d < 1e-12) return {};
  const ux = (bx - ax) / d;
  const uy = (by - ay) / d;
  const li = prev ? len(node.x - prev.x, node.y - prev.y) / 3 : 0;
  const lo = next ? len(next.x - node.x, next.y - node.y) / 3 : 0;
  return { ...(prev ? { in: [node.x - ux * li, node.y - uy * li] as Pt } : {}), ...(next ? { out: [node.x + ux * lo, node.y + uy * lo] as Pt } : {}) };
}

/** Recomputes the handles of auto nodes (after nodes moved). */
export function refreshAuto(subs: SubPath[]): SubPath[] {
  for (const sp of subs)
    sp.nodes.forEach((n, i) => {
      if (n.type !== 'auto') return;
      const h = autoHandles(sp, i);
      delete n.in;
      delete n.out;
      Object.assign(n, h);
    });
  return subs;
}

/** Sets the type of the chosen nodes, making their handles fit it. */
export function setNodeType(subs: readonly SubPath[], refs: readonly NodeRef[], type: NodeType): SubPath[] {
  const out = clone(subs);
  for (const r of refs) {
    const sp = out[r.sub];
    const node = sp?.nodes[r.index];
    if (!node) continue;
    node.type = type;
    if (type === 'cusp') continue;
    if (type === 'auto' || (!node.in && !node.out)) {
      const h = autoHandles(sp, r.index);
      delete node.in;
      delete node.out;
      Object.assign(node, h);
      continue;
    }
    const { prev, next } = neighbours(sp, r.index);
    // A missing handle is made opposite the other one, a third of its segment long.
    const inV: Pt = node.in ? [node.in[0] - node.x, node.in[1] - node.y] : [0, 0];
    const outV: Pt = node.out ? [node.out[0] - node.x, node.out[1] - node.y] : [0, 0];
    let li = len(inV[0], inV[1]);
    let lo = len(outV[0], outV[1]);
    if (!node.in && prev) li = len(node.x - prev.x, node.y - prev.y) / 3;
    if (!node.out && next) lo = len(next.x - node.x, next.y - node.y) / 3;
    // Direction: along out − in (both handles' average direction).
    let dx = outV[0] - inV[0];
    let dy = outV[1] - inV[1];
    if (len(dx, dy) < 1e-12) {
      dx = (next?.x ?? node.x) - (prev?.x ?? node.x);
      dy = (next?.y ?? node.y) - (prev?.y ?? node.y);
    }
    const d = len(dx, dy) || 1;
    const ux = dx / d;
    const uy = dy / d;
    if (type === 'symmetric') li = lo = (li + lo) / 2;
    if (prev || node.in) node.in = [node.x - ux * li, node.y - uy * li];
    if (next || node.out) node.out = [node.x + ux * lo, node.y + uy * lo];
  }
  return refreshAuto(out);
}

/** Moves the chosen nodes (their handles with them); auto nodes follow. */
export function moveNodes(subs: readonly SubPath[], refs: readonly NodeRef[], dx: number, dy: number): SubPath[] {
  const out = clone(subs);
  for (const r of refs) {
    const n = out[r.sub]?.nodes[r.index];
    if (!n) continue;
    n.x += dx;
    n.y += dy;
    if (n.in) n.in = [n.in[0] + dx, n.in[1] + dy];
    if (n.out) n.out = [n.out[0] + dx, n.out[1] + dy];
  }
  return refreshAuto(out);
}

/** Segments whose both ends are chosen: [sub, segment index]. */
function chosenSegments(subs: readonly SubPath[], refs: readonly NodeRef[]): [number, number][] {
  const on = new Set(refs.map(refKey));
  const out: [number, number][] = [];
  subs.forEach((sp, s) => {
    const n = segmentCount(sp);
    for (let i = 0; i < n; i++) if (on.has(refKey({ sub: s, index: i })) && on.has(refKey({ sub: s, index: (i + 1) % sp.nodes.length }))) out.push([s, i]);
  });
  return out;
}

/** A new node in the middle (t = ½) of every segment between two chosen nodes; the new nodes join the choice. */
export function insertMidNodes(subs: readonly SubPath[], refs: readonly NodeRef[]): { subs: SubPath[]; refs: NodeRef[] } {
  const segs = chosenSegments(subs, refs);
  const out = clone(subs);
  const added = new Map<number, number[]>();
  // From the last segment back, so earlier indices stay valid.
  for (const [s, i] of [...segs].sort((a, b) => b[0] - a[0] || b[1] - a[1])) {
    const sp = out[s];
    const a = sp.nodes[i];
    const bIdx = (i + 1) % sp.nodes.length;
    const b = sp.nodes[bIdx];
    let mid: PathNode;
    if (!a.out && !b.in) mid = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
    else {
      const [l, r] = splitCubic(segmentCubic(sp, i), 0.5);
      a.out = [l[1][0], l[1][1]];
      mid = { x: l[3][0], y: l[3][1], in: [l[2][0], l[2][1]], out: [r[1][0], r[1][1]] };
      b.in = [r[2][0], r[2][1]];
    }
    sp.nodes.splice(i + 1, 0, mid);
    added.set(s, [...(added.get(s) ?? []), i]);
  }
  // The choice: old nodes shifted past inserted ones, and the new ones.
  const next: NodeRef[] = [];
  out.forEach((_sp, s) => {
    const ins = (added.get(s) ?? []).sort((a, b) => a - b);
    const shift = (i: number) => i + ins.filter((k) => k < i).length;
    for (const r of refs) if (r.sub === s) next.push({ sub: s, index: shift(r.index) });
    ins.forEach((k, j) => next.push({ sub: s, index: k + 1 + j }));
  });
  return { subs: out, refs: next };
}

/**
 * Deletes the chosen nodes. With `keepShape` the curve over each removed
 * run is replaced by one cubic fitted to it (end tangents kept), as
 * Inkscape does; otherwise the neighbours are joined as they are. A
 * sub-path left with too few nodes goes away.
 */
export function deleteNodes(subs: readonly SubPath[], refs: readonly NodeRef[], keepShape = true): SubPath[] {
  const gone = new Set(refs.map(refKey));
  const out: SubPath[] = [];
  subs.forEach((sp, s) => {
    const n = sp.nodes.length;
    const dead = sp.nodes.map((_, i) => gone.has(refKey({ sub: s, index: i })));
    if (!dead.some(Boolean)) return void out.push({ closed: sp.closed, nodes: sp.nodes.map((x) => ({ ...x })) });
    const keep = sp.nodes.map((x, i) => (dead[i] ? null : { ...x }));
    const kept = keep.filter((x): x is PathNode => !!x);
    if (kept.length < 2 || (sp.closed && kept.length < 2)) return;
    if (keepShape) {
      // Each run of deleted nodes between two kept ones: one fitted cubic over the old curve.
      for (let i = 0; i < n; i++) {
        if (dead[i] || (!sp.closed && i === n - 1)) continue;
        if (!dead[(i + 1) % n]) continue;
        let j = (i + 1) % n;
        while (dead[j] && (sp.closed || j < n - 1)) j = (j + 1) % n;
        if (dead[j]) continue; // an open end was deleted: the run just goes
        const pts: Pt[] = [];
        let allLines = true;
        for (let k = i; k !== j; k = (k + 1) % n) {
          const c = segmentCubic(sp, k);
          if (!segmentIsLine(sp, k)) allLines = false;
          for (let t = k === i ? 0 : 1; t <= 16; t++) pts.push(bez(c, t / 16));
        }
        if (allLines) {
          delete keep[i]!.out;
          delete keep[j]!.in;
          continue;
        }
        const t1 = bezTangent(segmentCubic(sp, i), 0);
        const t2 = bezTangent(segmentCubic(sp, (j - 1 + n) % n), 1);
        const c = fitOne(pts, t1, [-t2[0], -t2[1]]);
        if (c) {
          keep[i]!.out = [c[1][0], c[1][1]];
          keep[j]!.in = [c[2][0], c[2][1]];
        }
      }
    }
    const nodes = keep.filter((x): x is PathNode => !!x);
    if (!sp.closed) {
      // A deleted end leaves its neighbour as the end, without a handle past it.
      delete nodes[0].in;
      delete nodes[nodes.length - 1].out;
    }
    out.push({ closed: sp.closed && nodes.length > 2, nodes });
  });
  return refreshAuto(out);
}

const isEnd = (sp: SubPath, i: number) => !sp.closed && (i === 0 || i === sp.nodes.length - 1);

/** The sub-path turned so the given end is its last node (`atEnd`) or its first. */
function orient(sp: SubPath, i: number, atEnd: boolean): SubPath {
  const last = i === sp.nodes.length - 1;
  if (last === atEnd) return sp;
  const nodes = sp.nodes.map((n) => {
    const m: PathNode = { ...n, in: n.out, out: n.in };
    if (!m.in) delete m.in;
    if (!m.out) delete m.out;
    return m;
  });
  return { closed: false, nodes: nodes.reverse() };
}

/**
 * Joins two chosen end nodes: into one node at their middle (`merge`) or
 * with a straight segment between them. Ends of one sub-path close it;
 * ends of two sub-paths make one.
 */
export function joinEnds(subs: readonly SubPath[], refs: readonly NodeRef[], merge: boolean): { subs: SubPath[]; refs: NodeRef[] } | { error: string } {
  const ends = refs.filter((r) => subs[r.sub] && isEnd(subs[r.sub], r.index));
  if (ends.length !== 2) return { error: 'Birleştirmek için açık yolların iki uç düğümünü seçin (Shift ile ikincisini ekleyin).' };
  const [a, b] = ends;
  const out = clone(subs);
  const mid = (p: PathNode, q: PathNode) => [(p.x + q.x) / 2, (p.y + q.y) / 2] as const;
  if (a.sub === b.sub) {
    const sp = out[a.sub];
    if (sp.nodes.length < 2 || a.index === b.index) return { error: 'Aynı düğüm iki kez seçilmiş.' };
    const first = sp.nodes[0];
    const last = sp.nodes[sp.nodes.length - 1];
    if (merge && sp.nodes.length > 2) {
      const [mx, my] = mid(first, last);
      const nodes = sp.nodes;
      const lastNode = nodes.pop()!;
      shiftNode(first, mx - first.x, my - first.y);
      if (lastNode.in) first.in = [lastNode.in[0] + mx - lastNode.x, lastNode.in[1] + my - lastNode.y];
      delete first.type;
    }
    sp.closed = true;
    return { subs: refreshAuto(out), refs: [{ sub: a.sub, index: 0 }] };
  }
  const A = orient(out[a.sub], a.index, true);
  const B = orient(out[b.sub], b.index, false);
  const aEnd = A.nodes[A.nodes.length - 1];
  const bStart = B.nodes[0];
  let nodes: PathNode[];
  if (merge) {
    const [mx, my] = mid(aEnd, bStart);
    const joined: PathNode = { x: mx, y: my };
    if (aEnd.in) joined.in = [aEnd.in[0] + mx - aEnd.x, aEnd.in[1] + my - aEnd.y];
    if (bStart.out) joined.out = [bStart.out[0] + mx - bStart.x, bStart.out[1] + my - bStart.y];
    nodes = [...A.nodes.slice(0, -1), joined, ...B.nodes.slice(1)];
  } else nodes = [...A.nodes, ...B.nodes];
  const keepAt = Math.min(a.sub, b.sub);
  const drop = Math.max(a.sub, b.sub);
  out[keepAt] = { closed: false, nodes };
  out.splice(drop, 1);
  const at = merge ? A.nodes.length - 1 : A.nodes.length;
  return { subs: refreshAuto(out), refs: [{ sub: keepAt, index: at }] };
}

function shiftNode(n: PathNode, dx: number, dy: number): void {
  n.x += dx;
  n.y += dy;
  if (n.in) n.in = [n.in[0] + dx, n.in[1] + dy];
  if (n.out) n.out = [n.out[0] + dx, n.out[1] + dy];
}

/** Breaks the path at the chosen nodes: a closed sub-path opens there, an open one splits in two. */
export function breakAtNodes(subs: readonly SubPath[], refs: readonly NodeRef[]): SubPath[] {
  const out: SubPath[] = [];
  subs.forEach((sp, s) => {
    const at = refs.filter((r) => r.sub === s).map((r) => r.index);
    let pieces: SubPath[] = [{ closed: sp.closed, nodes: sp.nodes.map((n) => ({ ...n })) }];
    if (sp.closed && at.length) {
      // Open at the first chosen node: it becomes both ends.
      const k = at[0];
      const nodes = [...sp.nodes.slice(k), ...sp.nodes.slice(0, k)].map((n) => ({ ...n }));
      const end: PathNode = { x: nodes[0].x, y: nodes[0].y };
      if (nodes[0].in) end.in = nodes[0].in;
      delete nodes[0].in;
      nodes.push(end);
      pieces = [{ closed: false, nodes }];
      // The other chosen nodes, re-indexed in the opened path.
      const n = sp.nodes.length;
      const rest = at.slice(1).map((i) => (i - k + n) % n);
      pieces = splitOpen(pieces[0], rest);
    } else if (!sp.closed) pieces = splitOpen(pieces[0], at);
    out.push(...pieces);
  });
  return out;
}

function splitOpen(sp: SubPath, at: readonly number[]): SubPath[] {
  const cuts = [...new Set(at)].filter((i) => i > 0 && i < sp.nodes.length - 1).sort((a, b) => a - b);
  if (!cuts.length) return [sp];
  const out: SubPath[] = [];
  let from = 0;
  for (const c of [...cuts, sp.nodes.length - 1]) {
    const nodes = sp.nodes.slice(from, c + 1).map((n) => ({ ...n }));
    delete nodes[0].in;
    delete nodes[nodes.length - 1].out;
    out.push({ closed: false, nodes });
    from = c;
  }
  return out;
}

/** Removes the segments between chosen neighbours: the path opens or splits there. */
export function deleteSegments(subs: readonly SubPath[], refs: readonly NodeRef[]): SubPath[] | { error: string } {
  const segs = chosenSegments(subs, refs);
  if (!segs.length) return { error: 'Silinecek parçanın iki ucundaki düğümleri seçin.' };
  const out: SubPath[] = [];
  subs.forEach((sp, s) => {
    const mine = segs.filter(([x]) => x === s).map(([, i]) => i);
    if (!mine.length) return void out.push({ closed: sp.closed, nodes: sp.nodes.map((n) => ({ ...n })) });
    const n = sp.nodes.length;
    // Walk the sub-path from just after a removed segment, starting a new piece after each one.
    const start = sp.closed ? (mine[0] + 1) % n : 0;
    const count = sp.closed ? n : n;
    let cur: PathNode[] = [];
    for (let k = 0; k < count; k++) {
      const i = (start + k) % n;
      cur.push({ ...sp.nodes[i] });
      const segRemoved = mine.includes(i) && (sp.closed || i < n - 1);
      const lastNode = !sp.closed && i === n - 1;
      if (segRemoved || lastNode || (sp.closed && k === count - 1)) {
        if (cur.length >= 2) {
          delete cur[0].in;
          delete cur[cur.length - 1].out;
          out.push({ closed: false, nodes: cur });
        }
        cur = [];
      }
    }
  });
  return out;
}

/** The segments between chosen neighbours as straight lines or as (straight-looking) curves ready to bend. */
export function segmentsTo(subs: readonly SubPath[], refs: readonly NodeRef[], kind: 'line' | 'curve'): SubPath[] {
  const out = clone(subs);
  for (const [s, i] of chosenSegments(subs, refs)) {
    const sp = out[s];
    const a = sp.nodes[i];
    const b = sp.nodes[(i + 1) % sp.nodes.length];
    if (kind === 'line') {
      delete a.out;
      delete b.in;
      if (a.type && a.type !== 'cusp') a.type = 'cusp';
      if (b.type && b.type !== 'cusp') b.type = 'cusp';
    } else if (!a.out && !b.in) {
      a.out = [a.x + (b.x - a.x) / 3, a.y + (b.y - a.y) / 3];
      b.in = [a.x + (2 * (b.x - a.x)) / 3, a.y + (2 * (b.y - a.y)) / 3];
    }
  }
  return out;
}

// ── Corners ────────────────────────────────────────────────────────────

/** What a corner node offers a fillet or chamfer: its angle and the longest cut along each side. */
export interface Corner {
  /** Angle between the two sides (radians, 0..π; π is straight on). */
  angle: number;
  /** Unit directions from the node along the incoming and outgoing sides. */
  back: Pt;
  ahead: Pt;
  /** Longest distance the cut may reach along each side (the neighbours' distance). */
  max: number;
}

export function cornerAt(sp: SubPath, i: number): Corner | null {
  if (segmentCount(sp) < 2) return null;
  if (!sp.closed && (i === 0 || i === sp.nodes.length - 1)) return null;
  const inSeg = segmentCubic(sp, (i - 1 + sp.nodes.length) % sp.nodes.length);
  const outSeg = segmentCubic(sp, i);
  const t1 = bezTangent(inSeg, 1);
  const t2 = bezTangent(outSeg, 0);
  const back: Pt = [-t1[0], -t1[1]];
  const angle = Math.acos(Math.max(-1, Math.min(1, back[0] * t2[0] + back[1] * t2[1])));
  const chord = (c: Cubic) => len(c[3][0] - c[0][0], c[3][1] - c[0][1]);
  return { angle, back, ahead: t2, max: Math.min(chord(inSeg), chord(outSeg)) };
}

/** Tangent distance (from the corner along each side) of a fillet of radius r. */
export const filletDistance = (c: Corner, r: number) => r / Math.tan(c.angle / 2);
/** Radius of the fillet reaching distance d along each side. */
export const filletRadius = (c: Corner, d: number) => d * Math.tan(c.angle / 2);

/**
 * Rounds (fillet: `size` is the radius) or cuts (chamfer: `size` is the
 * distance along each side) the corner at the chosen nodes. Sides may be
 * curves; the cut points are where the sides are that far from the
 * corner, and the fillet is the circular arc tangent to both.
 */
export function cornerNodes(subs: readonly SubPath[], refs: readonly NodeRef[], mode: 'fillet' | 'chamfer', size: number): { subs: SubPath[]; refs: NodeRef[] } | { error: string } {
  if (!(size > 0)) return { error: mode === 'fillet' ? 'Yarıçap sıfırdan büyük olmalı.' : 'Pah boyu sıfırdan büyük olmalı.' };
  const out = clone(subs);
  const made: NodeRef[] = [];
  let done = 0;
  let problem = '';
  const bySub = new Map<number, number[]>();
  for (const r of refs) bySub.set(r.sub, [...(bySub.get(r.sub) ?? []), r.index]);
  for (const [s, list] of bySub) {
    // From the last node back, so earlier indices stay valid.
    for (const i of [...new Set(list)].sort((a, b) => b - a)) {
      const sp = out[s];
      const c = cornerAt(sp, i);
      if (!c) {
        problem = 'Açık yolun uç düğümü köşe değildir.';
        continue;
      }
      if (c.angle > Math.PI - 1e-3) {
        problem = 'Düz devam eden bir düğüm köşe değildir.';
        continue;
      }
      if (c.angle < 1e-3) {
        problem = 'Geri dönen bir köşe yuvarlanamaz.';
        continue;
      }
      const d = mode === 'fillet' ? filletDistance(c, size) : size;
      const r = cutCorner(sp, i, d, mode, c);
      if ('error' in r) {
        problem = r.error;
        continue;
      }
      sp.nodes = r.nodes;
      done++;
      made.push({ sub: s, index: r.at }, { sub: s, index: r.at + 1 });
    }
  }
  if (!done) return { error: problem || 'Yuvarlanacak bir köşe düğümü seçin.' };
  return { subs: refreshAuto(out), refs: made };
}

function cutCorner(sp: SubPath, i: number, d: number, mode: 'fillet' | 'chamfer', c: Corner): { nodes: PathNode[]; at: number } | { error: string } {
  const n = sp.nodes.length;
  const pi = (i - 1 + n) % n;
  const inSeg = segmentCubic(sp, pi);
  const outSeg = segmentCubic(sp, i);
  const tA = paramAtDistance(inSeg, d, true);
  const tB = paramAtDistance(outSeg, d, false);
  if (tA === null || tB === null) return { error: mode === 'fillet' ? 'Yarıçap bu köşe için çok büyük.' : 'Pah boyu bu köşe için çok büyük.' };
  const inLine = segmentIsLine(sp, pi);
  const outLine = segmentIsLine(sp, i);
  // Straight sides are cut exactly (no search along them).
  const node = sp.nodes[i];
  const pA: Pt = [node.x + c.back[0] * d, node.y + c.back[1] * d];
  const pB: Pt = [node.x + c.ahead[0] * d, node.y + c.ahead[1] * d];
  const keepA: Cubic = inLine ? [inSeg[0], inSeg[0], pA, pA] : splitCubic(inSeg, tA)[0];
  const keepB: Cubic = outLine ? [pB, pB, outSeg[3], outSeg[3]] : splitCubic(outSeg, tB)[1];
  const nodes = sp.nodes.map((x) => ({ ...x }));
  const prev = nodes[pi];
  const next = nodes[(i + 1) % n];
  if (!inLine) prev.out = [keepA[1][0], keepA[1][1]];
  if (!outLine) next.in = [keepB[2][0], keepB[2][1]];
  const P: PathNode = { x: keepA[3][0], y: keepA[3][1] };
  const Q: PathNode = { x: keepB[0][0], y: keepB[0][1] };
  if (!inLine) P.in = [keepA[2][0], keepA[2][1]];
  if (!outLine) Q.out = [keepB[1][0], keepB[1][1]];
  if (mode === 'fillet') {
    // Tangents at the cut points, toward the corner and away from it; a circular arc's handles.
    const ta = bezTangent(keepA, 1);
    const tb = bezTangent(keepB, 0);
    const turn = Math.acos(Math.max(-1, Math.min(1, ta[0] * tb[0] + ta[1] * tb[1])));
    const chord = len(Q.x - P.x, Q.y - P.y);
    const r = turn > 1e-9 ? chord / (2 * Math.sin(turn / 2)) : 0;
    const k = (4 / 3) * Math.tan(turn / 4) * r;
    P.out = [P.x + ta[0] * k, P.y + ta[1] * k];
    Q.in = [Q.x - tb[0] * k, Q.y - tb[1] * k];
  }
  // A cut reaching a neighbour node merges into it.
  const same = (a: PathNode, b: PathNode) => len(a.x - b.x, a.y - b.y) < 1e-9;
  const list = [...nodes];
  list.splice(i, 1, P, Q);
  if (same(P, prev)) {
    if (P.out) prev.out = P.out;
    else delete prev.out;
    list.splice(list.indexOf(P), 1);
  }
  const q = list.indexOf(Q);
  const after = list[(q + 1) % list.length];
  if (after !== Q && same(Q, after)) {
    if (Q.in) after.in = Q.in;
    else delete after.in;
    list.splice(q, 1);
  }
  const first = list.includes(P) ? list.indexOf(P) : list.indexOf(prev);
  return { nodes: list, at: Math.max(0, first) };
}

// ── Align and distribute nodes ─────────────────────────────────────────

/** Moves the chosen nodes onto one line: their smallest, middle or largest x (or y). */
export function alignNodes(subs: readonly SubPath[], refs: readonly NodeRef[], axis: 'x' | 'y', to: 'min' | 'mid' | 'max'): SubPath[] {
  const pts = refs.map((r) => subs[r.sub]?.nodes[r.index]).filter((n): n is PathNode => !!n);
  if (pts.length < 2) return clone(subs);
  const vals = pts.map((n) => n[axis]);
  const lo = Math.min(...vals);
  const hi = Math.max(...vals);
  const target = to === 'min' ? lo : to === 'max' ? hi : (lo + hi) / 2;
  const out = clone(subs);
  for (const r of refs) {
    const n = out[r.sub]?.nodes[r.index];
    if (!n) continue;
    const d = target - n[axis];
    shiftNode(n, axis === 'x' ? d : 0, axis === 'y' ? d : 0);
  }
  return refreshAuto(out);
}

/** Spaces the chosen nodes evenly between the outermost ones along x (or y). */
export function distributeNodes(subs: readonly SubPath[], refs: readonly NodeRef[], axis: 'x' | 'y'): SubPath[] {
  const out = clone(subs);
  const list = refs.map((r) => out[r.sub]?.nodes[r.index]).filter((n): n is PathNode => !!n);
  if (list.length < 3) return out;
  const sorted = [...list].sort((a, b) => a[axis] - b[axis]);
  const lo = sorted[0][axis];
  const step = (sorted[sorted.length - 1][axis] - lo) / (sorted.length - 1);
  sorted.forEach((n, k) => {
    const d = lo + step * k - n[axis];
    shiftNode(n, axis === 'x' ? d : 0, axis === 'y' ? d : 0);
  });
  return refreshAuto(out);
}
