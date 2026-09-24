import { nearestOnCubic, segmentCount, segmentCubic, splitCubic } from '../../style/svg/bezier';
import { cornerAt, cornerNodes, filletRadius, nodeTypeOf, moveNodes, refKey, refreshAuto, type Corner, type NodeRef } from '../../style/svg/nodeOps';
import type { PathNode, Pt, SubPath } from '../../style/svg/pathData';
import type { SvgShape } from '../../style/svg/svgModel';
import { el, fmtNum, tag, type CanvasView } from './svgView';

/**
 * The node tool of the SVG editor (Inkscape's, with the CAD corner tools):
 * click a node to choose it (Shift adds), drag a box round nodes, drag the
 * chosen nodes together (snapping the grabbed one), drag a handle (smooth
 * nodes keep them in line, symmetric ones equal, auto ones turn smooth;
 * Alt frees it). A click on a segment chooses its two ends, a double click
 * adds a node there, a double click on a node switches cusp and smooth.
 * In the corner modes a corner under the pointer gets a ring: press on it
 * and pull along a side, the fillet radius or chamfer length follows, and
 * the release applies it.
 */

type PathShape = Extract<SvgShape, { kind: 'path' }>;

type Op =
  | { kind: 'nodes'; grab: NodeRef; p0: Pt; orig: SubPath[]; moved: boolean }
  | { kind: 'handle'; ref: NodeRef; part: 'in' | 'out'; orig: SubPath[] }
  | { kind: 'box'; p0: Pt; p1: Pt; add: boolean }
  | { kind: 'corner'; ref: NodeRef; corner: Corner; orig: SubPath[]; size: number };

export type CornerMode = 'fillet' | 'chamfer';

const HIT_PX = 7;

export class NodeTool {
  private readonly view: CanvasView;
  private op: Op | null = null;
  private forShape: string | null = null;
  private chosen: NodeRef[] = [];
  /** Corner mode (fillet or chamfer by dragging), and the last size used. */
  mode: CornerMode | null = null;
  size = 2;
  private hover: NodeRef | null = null;
  private preview: SubPath[] | null = null;

  constructor(view: CanvasView) {
    this.view = view;
  }

  /** The path being edited. */
  get shape(): PathShape | null {
    const id = this.view.host.nodeEdit;
    const s = id ? this.view.host.doc.shapes.find((x) => x.id === id) : undefined;
    return s?.kind === 'path' ? s : null;
  }

  /** The chosen nodes of the edited path (none after switching paths). */
  get selected(): NodeRef[] {
    const s = this.shape;
    if (!s || s.id !== this.forShape) return [];
    return this.chosen.filter((r) => s.subs[r.sub]?.nodes[r.index]);
  }

  setSelected(refs: readonly NodeRef[]): void {
    this.forShape = this.shape?.id ?? null;
    this.chosen = [...refs];
    this.view.host.nodesChanged?.();
  }

  get busy(): boolean {
    return !!this.op;
  }

  setMode(mode: CornerMode | null): void {
    this.mode = mode;
    this.hover = null;
    this.preview = null;
    const h = this.view.host;
    if (mode) h.status(`${mode === 'fillet' ? 'Köşe yuvarla' : 'Pah kır'}: köşe düğümüne basıp bir kenar boyunca çekin, bırakınca uygulanır; yazılı değer sağdaki alandan. Esc bitirir.`);
    this.view.render();
  }

  /** Replaces the edited path's sub-paths as one undo step (the node panel's operations). */
  apply(label: string, subs: SubPath[], refs: NodeRef[] = []): void {
    const s = this.shape;
    if (!s) return;
    const host = this.view.host;
    host.begin();
    s.subs = subs;
    // A path with no nodes left goes away.
    if (!subs.some((sp) => sp.nodes.length)) {
      host.doc.shapes = host.doc.shapes.filter((x) => x.id !== s.id);
      host.commit(label);
      host.editNodes(null);
      return;
    }
    this.setSelected(refs);
    host.commit(label);
  }

  // ── Pointer ──────────────────────────────────────────────────────────

  down(e: PointerEvent, p: Pt, target: Element): boolean {
    const s = this.shape;
    const host = this.view.host;
    if (!s) return false;
    if (this.mode) {
      const ref = this.cornerNear(p);
      if (!ref) return true;
      const c = cornerAt(s.subs[ref.sub], ref.index);
      if (!c) return true;
      if (!this.selected.some((r) => refKey(r) === refKey(ref))) this.setSelected([ref]);
      this.op = { kind: 'corner', ref, corner: c, orig: structuredClone(s.subs), size: 0 };
      return true;
    }
    const attr = target.getAttribute('data-node');
    if (attr) {
      const [si, ni, part] = attr.split(',');
      const ref = { sub: +si, index: +ni };
      if (part === 'node') {
        const on = this.selected.some((r) => refKey(r) === refKey(ref));
        if (e.shiftKey) {
          this.setSelected(on ? this.selected.filter((r) => refKey(r) !== refKey(ref)) : [...this.selected, ref]);
          this.view.render();
          return true;
        }
        if (!on) this.setSelected([ref]);
        host.begin();
        this.op = { kind: 'nodes', grab: ref, p0: p, orig: structuredClone(s.subs), moved: false };
      } else {
        host.begin();
        this.op = { kind: 'handle', ref, part: part as 'in' | 'out', orig: structuredClone(s.subs) };
      }
      this.view.render();
      return true;
    }
    const seg = this.segmentAt(p);
    if (seg) {
      // A segment: its two ends are chosen (Shift adds them).
      const sp = s.subs[seg.sub];
      const ends = [
        { sub: seg.sub, index: seg.seg },
        { sub: seg.sub, index: (seg.seg + 1) % sp.nodes.length },
      ];
      this.setSelected(e.shiftKey ? [...this.selected, ...ends.filter((r) => !this.selected.some((x) => refKey(x) === refKey(r)))] : ends);
      this.view.render();
      return true;
    }
    const id = target.closest('[data-id]')?.getAttribute('data-id');
    const other = id && id !== s.id ? host.doc.shapes.find((x) => x.id === id) : undefined;
    if (other && !other.locked) {
      // Another shape: a path is edited instead; anything else is selected.
      if (other.kind === 'path') host.editNodes(other.id);
      else {
        host.editNodes(null);
        host.select([other.id]);
      }
      return true;
    }
    this.op = { kind: 'box', p0: p, p1: p, add: e.shiftKey };
    return true;
  }

  move(e: PointerEvent, p: Pt): boolean {
    const s = this.shape;
    const op = this.op;
    if (!s) return false;
    if (!op) {
      if (this.mode) {
        const h = this.cornerNear(p);
        if (refKey(h ?? { sub: -1, index: -1 }) !== refKey(this.hover ?? { sub: -1, index: -1 })) {
          this.hover = h;
          this.view.render();
        }
        return true;
      }
      return false;
    }
    const host = this.view.host;
    switch (op.kind) {
      case 'nodes': {
        const o = op.orig[op.grab.sub].nodes[op.grab.index];
        if (!op.moved && Math.hypot(p[0] - op.p0[0], p[1] - op.p0[1]) * this.view.scale < 3) return true;
        op.moved = true;
        const q = this.view.snap([o.x + p[0] - op.p0[0], o.y + p[1] - op.p0[1]], { nodes: { shape: s.id, refs: this.selected } });
        s.subs = moveNodes(op.orig, this.selected, q[0] - o.x, q[1] - o.y);
        host.changed();
        return true;
      }
      case 'handle': {
        const q = this.view.snap(p, { nodes: { shape: s.id, refs: [op.ref] }, noGrid: true });
        s.subs = dragHandle(op.orig, op.ref, op.part, q, e.altKey);
        host.changed();
        return true;
      }
      case 'box':
        op.p1 = p;
        this.view.render();
        return true;
      case 'corner': {
        const n = op.orig[op.ref.sub].nodes[op.ref.index];
        const v: Pt = [p[0] - n.x, p[1] - n.y];
        const along = Math.max(v[0] * op.corner.back[0] + v[1] * op.corner.back[1], v[0] * op.corner.ahead[0] + v[1] * op.corner.ahead[1]);
        const d = Math.max(0, Math.min(op.corner.max, along));
        op.size = niceRound(this.mode === 'fillet' ? filletRadius(op.corner, d) : d, 2 / this.view.scale);
        const r = op.size > 0 ? cornerNodes(op.orig, this.selected, this.mode!, op.size) : null;
        this.preview = r && !('error' in r) ? r.subs : null;
        host.status(r && 'error' in r ? r.error : `${this.mode === 'fillet' ? 'Yarıçap' : 'Pah'} ${fmtNum(op.size)}${this.selected.length > 1 ? ` (${this.selected.length} köşe)` : ''}`, r && 'error' in r ? 'warn' : 'ok');
        this.view.render();
        return true;
      }
    }
  }

  up(): boolean {
    const op = this.op;
    this.op = null;
    if (!op) return false;
    const host = this.view.host;
    const s = this.shape;
    switch (op.kind) {
      case 'nodes':
        host.commit(op.moved ? 'Düğümü taşı' : '');
        return true;
      case 'handle':
        host.commit('Kolu taşı');
        return true;
      case 'box': {
        if (!s) return true;
        const x0 = Math.min(op.p0[0], op.p1[0]);
        const x1 = Math.max(op.p0[0], op.p1[0]);
        const y0 = Math.min(op.p0[1], op.p1[1]);
        const y1 = Math.max(op.p0[1], op.p1[1]);
        const inside: NodeRef[] = [];
        s.subs.forEach((sp, si) => sp.nodes.forEach((n, ni) => n.x >= x0 && n.x <= x1 && n.y >= y0 && n.y <= y1 && inside.push({ sub: si, index: ni })));
        const keep = op.add ? this.selected : [];
        this.setSelected([...keep, ...inside.filter((r) => !keep.some((k) => refKey(k) === refKey(r)))]);
        this.view.render();
        return true;
      }
      case 'corner': {
        this.preview = null;
        if (!(op.size > 0) || !s) {
          this.view.render();
          return true;
        }
        const r = cornerNodes(op.orig, this.selected, this.mode!, op.size);
        if ('error' in r) {
          host.status(r.error, 'warn');
          this.view.render();
          return true;
        }
        this.size = op.size;
        this.apply(this.mode === 'fillet' ? 'Köşe yuvarla' : 'Pah kır', r.subs, r.refs);
        host.status(`${this.mode === 'fillet' ? 'Yarıçap' : 'Pah'} ${fmtNum(op.size)} uygulandı. Sonraki köşeye basın ya da Esc.`);
        return true;
      }
    }
  }

  /** Double click: on a node, cusp ↔ smooth; on a segment, a new node there. */
  dbl(p: Pt, target: Element): boolean {
    const s = this.shape;
    if (!s) return false;
    const attr = target.getAttribute('data-node');
    if (attr?.endsWith(',node')) {
      const [si, ni] = attr.split(',').map(Number);
      const subs = structuredClone(s.subs);
      const sp = subs[si];
      const n = sp.nodes[ni];
      if (nodeTypeOf(sp, ni) === 'cusp') {
        // Handles along the neighbours' chord, a sixth of it each way.
        const len = sp.nodes.length;
        const prev = sp.nodes[(ni - 1 + len) % len];
        const next = sp.nodes[(ni + 1) % len];
        const dx = (next.x - prev.x) / 6;
        const dy = (next.y - prev.y) / 6;
        n.in = [n.x - dx, n.y - dy];
        n.out = [n.x + dx, n.y + dy];
        n.type = 'smooth';
      } else {
        delete n.in;
        delete n.out;
        n.type = 'cusp';
      }
      this.apply('Düğüm türü', subs, [{ sub: si, index: ni }]);
      return true;
    }
    const seg = this.segmentAt(p);
    if (!seg) return false;
    const subs = structuredClone(s.subs);
    const sp = subs[seg.sub];
    const a = sp.nodes[seg.seg];
    const bi = (seg.seg + 1) % sp.nodes.length;
    const b = sp.nodes[bi];
    let mid: PathNode;
    if (!a.out && !b.in) mid = { x: a.x + (b.x - a.x) * seg.t, y: a.y + (b.y - a.y) * seg.t };
    else {
      const [l, r] = splitCubic(segmentCubic(sp, seg.seg), seg.t);
      a.out = [l[1][0], l[1][1]];
      mid = { x: l[3][0], y: l[3][1], in: [l[2][0], l[2][1]], out: [r[1][0], r[1][1]] };
      b.in = [r[2][0], r[2][1]];
    }
    sp.nodes.splice(seg.seg + 1, 0, mid);
    this.apply('Düğüm ekle', refreshAuto(subs), [{ sub: seg.sub, index: seg.seg + 1 }]);
    return true;
  }

  /** Esc: the corner mode ends first, then the node choice. */
  cancel(): boolean {
    if (this.op) {
      const s = this.shape;
      if (s && this.op.kind !== 'box') s.subs = this.op.orig;
      this.op = null;
      this.preview = null;
      this.view.host.commit('');
      return true;
    }
    if (this.mode) {
      this.setMode(null);
      this.view.host.status('');
      return true;
    }
    if (this.selected.length) {
      this.setSelected([]);
      this.view.render();
      return true;
    }
    return false;
  }

  // ── Hits ─────────────────────────────────────────────────────────────

  private segmentAt(p: Pt): { sub: number; seg: number; t: number } | null {
    const s = this.shape;
    if (!s) return null;
    const tol = HIT_PX / this.view.scale;
    let best: { sub: number; seg: number; t: number; d: number } | null = null;
    s.subs.forEach((sp, si) => {
      for (let i = 0; i < segmentCount(sp); i++) {
        const c = segmentCubic(sp, i);
        const minX = Math.min(c[0][0], c[1][0], c[2][0], c[3][0]) - tol;
        const maxX = Math.max(c[0][0], c[1][0], c[2][0], c[3][0]) + tol;
        const minY = Math.min(c[0][1], c[1][1], c[2][1], c[3][1]) - tol;
        const maxY = Math.max(c[0][1], c[1][1], c[2][1], c[3][1]) + tol;
        if (p[0] < minX || p[0] > maxX || p[1] < minY || p[1] > maxY) continue;
        const h = nearestOnCubic(c, p);
        if (h.d <= tol && (!best || h.d < best.d)) best = { sub: si, seg: i, t: h.t, d: h.d };
      }
    });
    const b = best as { sub: number; seg: number; t: number; d: number } | null;
    return b && b.t > 1e-6 && b.t < 1 - 1e-6 ? b : null;
  }

  /** The corner node within reach of the pointer (corner modes). */
  private cornerNear(p: Pt): NodeRef | null {
    const s = this.shape;
    if (!s) return null;
    const tol = 12 / this.view.scale;
    let best: NodeRef | null = null;
    let bestD = tol;
    s.subs.forEach((sp, si) =>
      sp.nodes.forEach((n, ni) => {
        const d = Math.hypot(n.x - p[0], n.y - p[1]);
        if (d > bestD) return;
        const c = cornerAt(sp, ni);
        if (!c || c.angle > Math.PI - 1e-3 || c.angle < 1e-3) return;
        bestD = d;
        best = { sub: si, index: ni };
      }),
    );
    return best;
  }

  // ── Drawing ──────────────────────────────────────────────────────────

  draw(g: SVGElement): void {
    const s = this.shape;
    if (!s) return;
    const view = this.view;
    const t = (q: Pt) => view.toScreen(q);
    if (this.preview) g.append(el('path', { d: screenPath(this.preview, t), class: 'svge__ghost' }));
    const on = new Set(this.selected.map(refKey));
    const total = s.subs.reduce((k, sp) => k + sp.nodes.length, 0);
    // Handles: all of them on small paths; on big ones those of chosen nodes and the segments beside them.
    const showAll = total <= 24;
    const handleOf = new Set<string>();
    if (!showAll)
      for (const r of this.selected) {
        const sp = s.subs[r.sub];
        const len = sp.nodes.length;
        handleOf.add(`${r.sub}:${r.index}:in`).add(`${r.sub}:${r.index}:out`);
        if (r.index > 0 || sp.closed) handleOf.add(`${r.sub}:${(r.index - 1 + len) % len}:out`);
        if (r.index < len - 1 || sp.closed) handleOf.add(`${r.sub}:${(r.index + 1) % len}:in`);
      }
    s.subs.forEach((sp, si) =>
      sp.nodes.forEach((n, ni) => {
        const p = t([n.x, n.y]);
        for (const part of ['in', 'out'] as const) {
          const h = n[part];
          if (!h || !(showAll || handleOf.has(`${si}:${ni}:${part}`))) continue;
          const q = t(h);
          g.append(el('line', { x1: p[0], y1: p[1], x2: q[0], y2: q[1], class: 'svge__hline' }));
          g.append(el('circle', { cx: q[0], cy: q[1], r: 4, class: 'svge__ctrl', 'data-node': `${si},${ni},${part}` }));
        }
      }),
    );
    // Nodes over handles: cusp a diamond, smooth and symmetric a square, auto a circle; chosen ones filled.
    s.subs.forEach((sp, si) =>
      sp.nodes.forEach((n, ni) => {
        const [x, y] = t([n.x, n.y]);
        const cls = `svge__node${on.has(`${si}:${ni}`) ? ' svge__node--on' : ''}`;
        const type = nodeTypeOf(sp, ni);
        const attrs = { class: cls, 'data-node': `${si},${ni},node` };
        if (type === 'cusp') g.append(el('path', { ...attrs, d: `M${x} ${y - 5.5}L${x + 5.5} ${y}L${x} ${y + 5.5}L${x - 5.5} ${y}Z` }));
        else if (type === 'auto') g.append(el('circle', { ...attrs, cx: x, cy: y, r: 4.5 }));
        else g.append(el('rect', { ...attrs, x: x - 4, y: y - 4, width: 8, height: 8 }));
      }),
    );
    if (this.op?.kind === 'box') {
      const a = t(this.op.p0);
      const b = t(this.op.p1);
      g.append(el('rect', { x: Math.min(a[0], b[0]), y: Math.min(a[1], b[1]), width: Math.abs(a[0] - b[0]), height: Math.abs(a[1] - b[1]), class: 'svge__marquee' }));
    }
    const ring = this.op?.kind === 'corner' ? this.op.ref : this.hover;
    if (this.mode && ring) {
      const n = s.subs[ring.sub]?.nodes[ring.index];
      if (n) {
        const [x, y] = t([n.x, n.y]);
        g.append(el('circle', { cx: x, cy: y, r: 11, class: 'svge__cornerring' }));
        if (this.op?.kind === 'corner' && this.op.size > 0) g.append(tag(x + 14, y - 12, `${this.mode === 'fillet' ? 'R ' : ''}${fmtNum(this.op.size)}`));
      }
    }
  }
}

/** A handle dragged to q, the other one kept as the node's type says. */
function dragHandle(orig: readonly SubPath[], ref: NodeRef, part: 'in' | 'out', q: Pt, free: boolean): SubPath[] {
  const subs = structuredClone(orig) as SubPath[];
  const sp = subs[ref.sub];
  const n = sp.nodes[ref.index];
  const type = nodeTypeOf(orig[ref.sub], ref.index);
  n[part] = [q[0], q[1]];
  const other = part === 'in' ? 'out' : 'in';
  const oh = n[other];
  if (free) {
    n.type = 'cusp';
    return refreshAuto(subs);
  }
  if (type === 'auto') n.type = 'smooth';
  if (!oh || type === 'cusp') return refreshAuto(subs);
  const dx = q[0] - n.x;
  const dy = q[1] - n.y;
  const l = Math.hypot(dx, dy) || 1;
  const len = type === 'symmetric' ? l : Math.hypot(oh[0] - n.x, oh[1] - n.y);
  n[other] = [n.x - (dx / l) * len, n.y - (dy / l) * len];
  return refreshAuto(subs);
}

/** A size rounded to a 1-2-5 step near `unit` (what a pixel is worth), so dragging gives tidy values. */
function niceRound(v: number, unit: number): number {
  const e = 10 ** Math.floor(Math.log10(unit));
  const step = unit / e < 2 ? e : unit / e < 5 ? 2 * e : 5 * e;
  return Math.round(v / step) * step;
}

/** Sub-paths as path data in screen space (previews). */
export function screenPath(subs: readonly SubPath[], t: (p: Pt) => Pt): string {
  const f = (p: Pt) => `${p[0].toFixed(1)} ${p[1].toFixed(1)}`;
  let d = '';
  for (const sp of subs) {
    const n = sp.nodes.length;
    if (!n) continue;
    d += `M${f(t([sp.nodes[0].x, sp.nodes[0].y]))}`;
    for (let i = 1; i < n + (sp.closed ? 1 : 0); i++) {
      const a = sp.nodes[i - 1];
      const b = sp.nodes[i % n];
      d += a.out || b.in ? `C${f(t(a.out ?? [a.x, a.y]))} ${f(t(b.in ?? [b.x, b.y]))} ${f(t([b.x, b.y]))}` : `L${f(t([b.x, b.y]))}`;
    }
    if (sp.closed) d += 'Z';
  }
  return d;
}
