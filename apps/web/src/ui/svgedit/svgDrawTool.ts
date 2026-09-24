import type { PathNode, Pt } from '../../style/svg/pathData';
import { regularPolygon, rotation, shapeId, transformShape, type Paint, type SvgShape } from '../../style/svg/svgModel';
import { screenPath } from './svgNodeTool';
import { el, type CanvasView } from './svgView';

/**
 * The SVG editor's drawing tools: shapes dragged out (rectangle, ellipse,
 * regular polygon or star, text), and the polyline and Bézier pen drafts
 * clicked node by node (the pen's drag gives a node symmetric handles; a
 * click on the first node closes, Enter or a double click ends it).
 */

const LABEL: Record<string, string> = { rect: 'Dikdörtgen', ellipse: 'Elips', polygon: 'Çokgen', text: 'Yazı' };

export class DrawTool {
  private readonly view: CanvasView;
  /** Nodes of the polyline or pen path being drawn. */
  private draft: PathNode[] = [];
  private cursor: Pt | null = null;
  /** Shift was held at the last pointer event (square, circle). */
  shift = false;

  constructor(view: CanvasView) {
    this.view = view;
  }

  get drafting(): boolean {
    return this.draft.length > 0;
  }

  /** The last node of the draft (perpendicular and tangent snaps start there). */
  get last(): Pt | null {
    const n = this.draft[this.draft.length - 1];
    return n ? [n.x, n.y] : null;
  }

  /** A click with the polyline or pen tool: a new node, or the end when it lands on the first node. */
  click(q: Pt): 'closed' | 'node' {
    const first = this.draft[0];
    if (first && this.draft.length > 2 && Math.hypot(first.x - q[0], first.y - q[1]) < 8 / this.view.scale) {
      this.finish(true);
      return 'closed';
    }
    this.draft.push({ x: q[0], y: q[1] });
    this.view.render();
    return 'node';
  }

  hover(q: Pt): void {
    this.cursor = q;
    this.view.render();
  }

  /** Dragging out of the pen's new node gives it symmetric handles. */
  penDrag(q: Pt): void {
    const n = this.draft[this.draft.length - 1];
    if (n && Math.hypot(q[0] - n.x, q[1] - n.y) > 2 / this.view.scale) {
      n.out = q;
      n.in = [2 * n.x - q[0], 2 * n.y - q[1]];
    }
  }

  /** The double click's second press added a node on the last one: it goes before the end. */
  dropLast(): void {
    if (this.draft.length > 1) this.draft.pop();
  }

  /** Ends the polyline or pen path being drawn (Enter, double click, a click on its first node). */
  finish(closed: boolean): void {
    const host = this.view.host;
    const nodes = this.draft;
    this.draft = [];
    this.cursor = null;
    if (nodes.length >= 2) {
      const shape: SvgShape = { id: shapeId(), kind: 'path', subs: [{ nodes, closed }], fill: closed ? 'fill' : 'none', stroke: closed ? 'none' : 'fill', strokeWidth: Math.max(1, host.doc.width / 25) };
      host.begin();
      host.doc.shapes.push(shape);
      host.commit(host.tool === 'pen' ? 'Kalem' : 'Çizgi');
      host.select([shape.id]);
    }
    this.view.render();
  }

  cancel(): boolean {
    if (!this.draft.length) return false;
    this.draft = [];
    this.cursor = null;
    this.view.render();
    return true;
  }

  /** The shape a drag from p0 to p1 makes with the current tool (Alt: from the centre). */
  shapeFromDrag(p0: Pt, p1: Pt, fromCentre: boolean): SvgShape | null {
    const host = this.view.host;
    const tool = host.tool;
    const base = { id: shapeId(), fill: 'fill' as Paint, stroke: 'none' as Paint, strokeWidth: Math.max(1, host.doc.width / 50) };
    if (tool === 'text') return { ...base, kind: 'text', x: p0[0], y: p0[1], text: 'Aa', size: host.doc.height / 5, weight: 700, font: 'sans', anchor: 'start' };
    let dx = p1[0] - p0[0];
    let dy = p1[1] - p0[1];
    if (Math.hypot(dx, dy) < 0.5) return null;
    if (tool === 'polygon') {
      const r = Math.hypot(dx, dy);
      const sp = regularPolygon(p0[0], p0[1], r, Math.max(3, host.options.sides), host.options.star ? r * 0.45 : undefined);
      // The first corner points at the pointer.
      const turn = Math.atan2(dy, dx) + Math.PI / 2;
      return transformShape({ ...base, kind: 'path', subs: [sp] }, rotation((turn * 180) / Math.PI, p0[0], p0[1]));
    }
    if (this.shift) {
      const k = Math.max(Math.abs(dx), Math.abs(dy));
      dx = Math.sign(dx || 1) * k;
      dy = Math.sign(dy || 1) * k;
    }
    const x0 = fromCentre ? p0[0] - Math.abs(dx) : Math.min(p0[0], p0[0] + dx);
    const y0 = fromCentre ? p0[1] - Math.abs(dy) : Math.min(p0[1], p0[1] + dy);
    const w = fromCentre ? 2 * Math.abs(dx) : Math.abs(dx);
    const h = fromCentre ? 2 * Math.abs(dy) : Math.abs(dy);
    if (tool === 'rect') return { ...base, kind: 'rect', x: x0, y: y0, w, h };
    return { ...base, kind: 'ellipse', cx: x0 + w / 2, cy: y0 + h / 2, rx: w / 2, ry: h / 2 };
  }

  /** A finished drag: the shape goes in as one undo step and is chosen (back to Seç, except for text). */
  place(p0: Pt, p1: Pt, fromCentre: boolean): void {
    const host = this.view.host;
    const shape = this.shapeFromDrag(p0, p1, fromCentre);
    if (!shape) return;
    host.begin();
    host.doc.shapes.push(shape);
    host.commit(LABEL[host.tool] ?? 'Şekil');
    host.select([shape.id]);
    if (host.tool !== 'text') host.setTool('select');
  }

  /** The draft (screen space). */
  draw(g: SVGElement): void {
    if (!this.draft.length) return;
    const t = (p: Pt) => this.view.toScreen(p);
    const pts = this.draft;
    g.append(el('path', { d: screenPath([{ closed: false, nodes: pts }], t), class: 'svge__draft' }));
    if (this.cursor) {
      const last = t([pts[pts.length - 1].x, pts[pts.length - 1].y]);
      const c = t(this.cursor);
      g.append(el('line', { x1: last[0], y1: last[1], x2: c[0], y2: c[1], class: 'svge__draft' }));
    }
    for (const n of pts) {
      const s = t([n.x, n.y]);
      g.append(el('rect', { x: s[0] - 3, y: s[1] - 3, width: 6, height: 6, class: 'svge__node' }));
    }
  }
}
