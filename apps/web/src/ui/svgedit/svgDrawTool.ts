import type { PathNode, Pt } from '../../style/svg/pathData';
import { shapeId, type SvgShape } from '../../style/svg/svgModel';
import { lineStrokeWidth, shapeFromDrag } from './svgEditModel';
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
      // `in` before `out`: the order the core writes nodes in (the editor compares drawings as text).
      n.in = [2 * n.x - q[0], 2 * n.y - q[1]];
      n.out = q;
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
      const shape: SvgShape = { id: shapeId(), kind: 'path', subs: [{ closed, nodes }], fill: closed ? 'fill' : 'none', stroke: closed ? 'none' : 'fill', strokeWidth: lineStrokeWidth(host.doc.width) };
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

  /** The shape a drag from p0 to p1 makes with the current tool (Alt: from the centre; svgEditModel.ts). */
  shapeFromDrag(p0: Pt, p1: Pt, fromCentre: boolean): SvgShape | null {
    const host = this.view.host;
    const tool = host.tool;
    if (tool !== 'rect' && tool !== 'ellipse' && tool !== 'polygon' && tool !== 'text') return null;
    const { width, height } = host.doc;
    return shapeFromDrag({ tool, width, height, sides: host.options.sides, star: host.options.star, shift: this.shift, fromCentre }, p0, p1, shapeId());
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
