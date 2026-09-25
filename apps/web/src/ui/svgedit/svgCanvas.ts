import type { Box, Matrix, Pt } from '../../style/svg/pathData';
import { boxToBox, elementOf, rotation, shapeBox, shapesBox, transformShape, translate, type Paint, type SvgDoc, type SvgShape } from '../../style/svg/svgModel';
import { DrawTool } from './svgDrawTool';
import { Measure } from './svgMeasure';
import { NodeTool } from './svgNodeTool';
import { RULER, Rulers } from './svgRulers';
import { Snapper } from './svgSnap';
import { el, type CanvasHost, type CanvasView, type SnapOptions } from './svgView';

export type { CanvasHost, CanvasOptions, ToolId } from './svgView';

/**
 * The SVG editor's drawing surface: the canvas (viewBox) on paper with a
 * grid, the shapes, and screen-space handles on top, rulers and guides at
 * the edges. Tools: select (move, scale by eight handles, rotate by the
 * top knob), nodes (svgNodeTool.ts), the drawing tools (svgDrawTool.ts:
 * rectangle, ellipse, polygon/star, polyline, Bézier pen, text) and
 * measure (svgMeasure.ts).
 * Points snap to shapes, guides and the canvas (svgSnap.ts), else to the
 * grid. Panels may ask for a point on the canvas (a rotation centre) and
 * show copies about to be made (array preview).
 */

type Op =
  | { kind: 'pan'; x: number; y: number; ox: number; oy: number }
  | { kind: 'move'; p0: Pt; orig: Map<string, SvgShape>; box: Box; sources: Pt[]; narrow: string[] | null; moved: boolean }
  | { kind: 'scale'; handle: number; p0: Pt; orig: Map<string, SvgShape>; box: Box; moved: boolean }
  | { kind: 'rotate'; p0: Pt; orig: Map<string, SvgShape>; box: Box; moved: boolean }
  | { kind: 'marquee'; p0: Pt; p1: Pt; add: boolean }
  | { kind: 'draw'; p0: Pt; p1: Pt };

/** Screen pixels the pointer travels before a press moves anything (the node tool's node drag uses the same). */
const DRAG_PX = 3;
/** Two presses this close in time (ms, as the drawing's select tool) and place (px) are a double click. */
const DOUBLE_MS = 450;
const CLICK_SLOP_PX = 4;

export class SvgCanvas implements CanvasView {
  readonly el: HTMLElement;
  readonly host: CanvasHost;
  readonly nodes: NodeTool;
  readonly rulers: Rulers;
  private readonly measure: Measure;
  private readonly drawing: DrawTool;
  private readonly snapper: Snapper;
  private readonly svg: SVGSVGElement;
  private zoom = 4;
  /** The left press under way (for double clicks), and the last press that was a click. */
  private press: { t: number; clientX: number; clientY: number; target: Element; travelled: boolean } | null = null;
  private lastClick: { t: number; clientX: number; clientY: number } | null = null;
  private ox = 20;
  private oy = 20;
  private op: Op | null = null;
  private spaceDown = false;
  private fitted = false;
  /** A panel waits for a point (rotation or array centre). */
  private picking: { prompt: string; done: (p: Pt) => void } | null = null;
  /** A point a panel shows (the chosen centre). */
  private marker: Pt | null = null;
  /** Copies a panel is about to make, drawn as ghosts. */
  private preview: { ids: ReadonlySet<string>; matrices: readonly Matrix[] } | null = null;

  constructor(host: CanvasHost) {
    this.host = host;
    this.svg = el('svg', { class: 'svge__svg' }) as SVGSVGElement;
    this.el = document.createElement('div');
    this.el.className = 'svge__stage';
    this.el.tabIndex = 0;
    this.el.append(this.svg);
    this.snapper = new Snapper(host);
    this.nodes = new NodeTool(this);
    this.rulers = new Rulers(this);
    this.measure = new Measure(this);
    this.drawing = new DrawTool(this);
    this.svg.addEventListener('pointerdown', (e) => this.down(e));
    this.svg.addEventListener('pointermove', (e) => this.move(e));
    this.svg.addEventListener('pointerup', (e) => this.up(e));
    this.svg.addEventListener('wheel', (e) => this.wheel(e), { passive: false });
    this.el.addEventListener('keydown', (e) => {
      if (e.key === ' ') this.spaceDown = true;
    });
    this.el.addEventListener('keyup', (e) => {
      if (e.key === ' ') this.spaceDown = false;
    });
    new ResizeObserver(() => {
      if (!this.fitted && this.el.clientWidth > 0) {
        this.fitted = true;
        this.fit();
      } else this.render();
    }).observe(this.el);
  }

  // ── View ─────────────────────────────────────────────────────────────

  get stage(): HTMLElement {
    return this.el;
  }

  fit(): void {
    const { width, height } = this.host.doc;
    const r = this.host.options.rulers ? RULER : 0;
    const w = (this.el.clientWidth || 600) - r;
    const h = (this.el.clientHeight || 500) - r;
    const k = this.host.options.tile ? 3 : 1;
    this.zoom = Math.min((w - 48) / (width * k), (h - 48) / (height * k));
    this.ox = r + (w - width * this.zoom) / 2;
    this.oy = r + (h - height * this.zoom) / 2;
    this.render();
    this.host.zoomed?.(this.zoom);
  }

  zoomBy(f: number, at?: Pt): void {
    const cx = at ? at[0] : this.el.clientWidth / 2;
    const cy = at ? at[1] : this.el.clientHeight / 2;
    const z = Math.min(200, Math.max(0.2, this.zoom * f));
    this.ox = cx - ((cx - this.ox) * z) / this.zoom;
    this.oy = cy - ((cy - this.oy) * z) / this.zoom;
    this.zoom = z;
    this.render();
    this.host.zoomed?.(this.zoom);
  }

  get scale(): number {
    return this.zoom;
  }

  toDoc(e: { clientX: number; clientY: number }): Pt {
    const r = this.svg.getBoundingClientRect();
    return [(e.clientX - r.left - this.ox) / this.zoom, (e.clientY - r.top - this.oy) / this.zoom];
  }

  toScreen(p: Pt): Pt {
    return [p[0] * this.zoom + this.ox, p[1] * this.zoom + this.oy];
  }

  private paint = (p: Paint): string => (p === 'fill' ? this.host.options.ink : p === 'stroke' ? this.host.options.second : p);

  snap(p: Pt, o: SnapOptions = {}): Pt {
    return this.snapper.snap(p, this.zoom, o);
  }

  /** The drawing, guides or snap options changed outside a drag: snap points are found anew. */
  invalidate(): void {
    this.snapper.reset();
  }

  // ── Requests from panels ─────────────────────────────────────────────

  /** The next click (snapped) gives a point; Esc gives up. */
  pickPoint(prompt: string, done: (p: Pt) => void): void {
    this.picking = { prompt, done };
    this.host.status(prompt);
    this.el.focus();
    this.render();
  }

  get isPicking(): boolean {
    return !!this.picking;
  }

  setMarker(p: Pt | null): void {
    this.marker = p;
    this.render();
  }

  setPreview(ids: ReadonlySet<string> | null, matrices: readonly Matrix[] | null): void {
    this.preview = ids && matrices?.length ? { ids, matrices } : null;
    this.render();
  }

  // ── Rendering ────────────────────────────────────────────────────────

  render(): void {
    const { doc, options: o } = this.host;
    const svg = this.svg;
    svg.replaceChildren();
    const world = el('g', { transform: `translate(${this.ox} ${this.oy}) scale(${this.zoom})` });
    world.append(el('rect', { x: 0, y: 0, width: doc.width, height: doc.height, fill: o.paper, class: 'svge__paper' }));
    this.host.underlay?.(world as SVGGElement);
    if (o.grid > 0 && o.grid * this.zoom >= 5) {
      let d = '';
      for (let x = 0; x <= doc.width + 1e-9; x += o.grid) d += `M${x} 0V${doc.height}`;
      for (let y = 0; y <= doc.height + 1e-9; y += o.grid) d += `M0 ${y}H${doc.width}`;
      world.append(el('path', { d, class: 'svge__grid', 'stroke-width': 1 / this.zoom }));
    }
    const shapes = el('g');
    for (const s of doc.shapes) if (!s.hidden) shapes.append(this.shapeEl(s));
    if (o.tile) {
      // The drawing repeated around itself, as a pattern fill tiles it.
      for (const dx of [-1, 0, 1])
        for (const dy of [-1, 0, 1]) {
          if (!dx && !dy) continue;
          const copy = shapes.cloneNode(true) as SVGGElement;
          copy.setAttribute('transform', `translate(${dx * doc.width} ${dy * doc.height})`);
          copy.setAttribute('opacity', '0.45');
          copy.setAttribute('pointer-events', 'none');
          world.append(copy);
        }
    }
    world.append(shapes);
    if (this.preview) world.append(this.ghosts(this.preview));
    world.append(el('rect', { x: 0, y: 0, width: doc.width, height: doc.height, fill: 'none', class: 'svge__frame', 'stroke-width': 1 / this.zoom, 'pointer-events': 'none' }));
    svg.append(world);
    svg.append(this.overlay());
  }

  private shapeEl(s: SvgShape): SVGElement {
    const spec = elementOf(s, this.paint);
    const node = el(spec.tag, spec.attrs);
    if (spec.text !== undefined) node.textContent = spec.text;
    node.setAttribute('data-id', s.id);
    // A locked shape lets clicks through to what is under it.
    node.setAttribute('pointer-events', s.locked ? 'none' : 'all');
    return node;
  }

  /** The copies an array or mirror would make (at most 400 shapes). */
  private ghosts(pv: { ids: ReadonlySet<string>; matrices: readonly Matrix[] }): SVGElement {
    const g = el('g', { class: 'svge__ghosts', 'pointer-events': 'none' });
    const src = this.host.doc.shapes.filter((s) => pv.ids.has(s.id) && !s.hidden);
    let count = 0;
    for (const m of pv.matrices)
      for (const s of src) {
        if (count++ > 400) return g;
        const spec = elementOf(transformShape(s, m), this.paint);
        const node = el(spec.tag, spec.attrs);
        if (spec.text !== undefined) node.textContent = spec.text;
        g.append(node);
      }
    return g;
  }

  private overlay(): SVGGElement {
    const g = el('g', { class: 'svge__overlay' }) as SVGGElement;
    const { doc, selection, tool } = this.host;
    this.rulers.drawGuides(g);
    this.drawing.draw(g);
    if (this.op?.kind === 'draw') this.drawPreview(g, this.op.p0, this.op.p1);
    if (this.op?.kind === 'marquee') {
      const a = this.toScreen(this.op.p0);
      const b = this.toScreen(this.op.p1);
      g.append(el('rect', { x: Math.min(a[0], b[0]), y: Math.min(a[1], b[1]), width: Math.abs(a[0] - b[0]), height: Math.abs(a[1] - b[1]), class: 'svge__marquee' }));
    }
    const sel = doc.shapes.filter((s) => selection.has(s.id));
    if (this.host.nodeEdit && this.nodes.shape) this.nodes.draw(g);
    else if (sel.length && tool === 'select') {
      const b = shapesBox(sel)!;
      const [x0, y0] = this.toScreen([b.minX, b.minY]);
      const [x1, y1] = this.toScreen([b.maxX, b.maxY]);
      g.append(el('rect', { x: x0, y: y0, width: x1 - x0, height: y1 - y0, class: 'svge__selbox' }));
      // A locked shape shows its box but no handles.
      if (!sel.some((s) => s.locked)) {
        const mx = (x0 + x1) / 2;
        const my = (y0 + y1) / 2;
        const handles: Pt[] = [
          [x0, y0],
          [mx, y0],
          [x1, y0],
          [x1, my],
          [x1, y1],
          [mx, y1],
          [x0, y1],
          [x0, my],
        ];
        handles.forEach(([hx, hy], i) => g.append(el('rect', { x: hx - 4, y: hy - 4, width: 8, height: 8, class: 'svge__handle', 'data-handle': i })));
        g.append(el('line', { x1: mx, y1: y0, x2: mx, y2: y0 - 22, class: 'svge__selbox' }));
        g.append(el('circle', { cx: mx, cy: y0 - 26, r: 5, class: 'svge__handle svge__rot', 'data-handle': 'rot' }));
      }
    }
    if (tool === 'measure') this.measure.draw(g);
    if (this.marker) {
      const [x, y] = this.toScreen(this.marker);
      g.append(el('path', { d: `M${x - 9} ${y}H${x + 9}M${x} ${y - 9}V${y + 9}M${x - 4} ${y}a4 4 0 1 0 8 0a4 4 0 1 0 -8 0`, class: 'svge__pin' }));
    }
    if (this.op || this.drawing.drafting || this.picking || tool === 'measure' || this.nodes.busy) this.snapper.draw(g, (p) => this.toScreen(p));
    this.rulers.drawRulers(g);
    return g;
  }

  private drawPreview(g: SVGGElement, p0: Pt, p1: Pt): void {
    const shape = this.drawing.shapeFromDrag(p0, p1, false);
    if (!shape) return;
    const spec = elementOf(shape, this.paint);
    const node = el(spec.tag, spec.attrs);
    if (spec.text !== undefined) node.textContent = spec.text;
    const w = el('g', { transform: `translate(${this.ox} ${this.oy}) scale(${this.zoom})`, opacity: 0.6 });
    w.append(node);
    g.append(w);
  }

  // ── Pointer ──────────────────────────────────────────────────────────

  private down(e: PointerEvent): void {
    this.el.focus();
    this.drawing.shift = e.shiftKey;
    this.snapper.reset();
    const p = this.toDoc(e);
    if (e.button === 1 || (e.button === 0 && this.spaceDown)) {
      this.op = { kind: 'pan', x: e.clientX, y: e.clientY, ox: this.ox, oy: this.oy };
      this.svg.setPointerCapture(e.pointerId);
      return;
    }
    if (e.button !== 0) return;
    this.svg.setPointerCapture(e.pointerId);
    const target = e.target as Element;
    this.press = { t: e.timeStamp, clientX: e.clientX, clientY: e.clientY, target, travelled: false };
    const host = this.host;
    if (this.picking) {
      const pick = this.picking;
      this.picking = null;
      pick.done(this.snap(p));
      this.snapper.hit = null;
      this.render();
      return;
    }
    if (this.rulers.down(p, target)) return;
    const tool = host.tool;
    if (tool === 'measure') return this.measure.down(p);
    if (host.nodeEdit && tool === 'node' && this.nodes.down(e, p, target)) return;
    const handle = target.getAttribute('data-handle');
    if (tool === 'select' && handle !== null) {
      const sel = host.doc.shapes.filter((s) => host.selection.has(s.id));
      const box = shapesBox(sel);
      if (!box) return;
      host.begin();
      const orig = new Map(sel.map((s) => [s.id, structuredClone(s)]));
      this.op = handle === 'rot' ? { kind: 'rotate', p0: p, orig, box, moved: false } : { kind: 'scale', handle: Number(handle), p0: p, orig, box, moved: false };
      return;
    }
    switch (tool) {
      case 'select':
      case 'node': {
        const id = target.closest('[data-id]')?.getAttribute('data-id');
        const shape = id ? host.doc.shapes.find((s) => s.id === id) : undefined;
        if (shape && !shape.locked) {
          if (tool === 'node' && shape.kind === 'path') return host.editNodes(shape.id);
          const members = this.groupOf(shape.id);
          if (e.shiftKey) {
            const next = new Set(host.selection);
            const on = members.every((m) => next.has(m));
            for (const m of members) on ? next.delete(m) : next.add(m);
            host.select([...next]);
            return;
          }
          // A click (no drag) on one of several chosen shapes chooses only it on release (Inkscape's way).
          const narrow = host.selection.has(shape.id) && host.selection.size > members.length ? members : null;
          if (!host.selection.has(shape.id)) host.select(members);
          const sel = host.doc.shapes.filter((s) => host.selection.has(s.id));
          if (sel.some((s) => s.locked)) return;
          host.begin();
          const box = shapesBox(sel)!;
          this.op = { kind: 'move', p0: p, orig: new Map(sel.map((s) => [s.id, structuredClone(s)])), box, sources: moveSources(sel, box), narrow, moved: false };
        } else {
          this.op = { kind: 'marquee', p0: p, p1: p, add: e.shiftKey };
          if (!e.shiftKey) host.select([]);
        }
        return;
      }
      case 'rect':
      case 'ellipse':
      case 'polygon':
      case 'text': {
        const q = this.snap(p);
        this.op = { kind: 'draw', p0: q, p1: q };
        return;
      }
      case 'line':
      case 'pen': {
        const q = this.snap(p, { from: this.drawing.last });
        if (this.drawing.click(q) === 'closed') return;
        this.op = tool === 'pen' ? { kind: 'draw', p0: q, p1: q } : null;
        this.render();
        return;
      }
    }
  }

  private move(e: PointerEvent): void {
    this.drawing.shift = e.shiftKey;
    const press = this.press;
    if (press && !press.travelled && Math.hypot(e.clientX - press.clientX, e.clientY - press.clientY) > CLICK_SLOP_PX) press.travelled = true;
    const p = this.toDoc(e);
    const op = this.op;
    const host = this.host;
    if (this.rulers.move(e, p)) return;
    if (host.tool === 'measure' && !op) return this.measure.move(p, e.target as Element);
    if (host.nodeEdit && host.tool === 'node' && !op && this.nodes.move(e, p)) return;
    if (!op) {
      if (this.drawing.drafting) this.drawing.hover(this.snap(p, { from: this.drawing.last }));
      else if (this.picking) {
        this.snap(p);
        this.render();
      }
      return;
    }
    switch (op.kind) {
      case 'pan':
        this.ox = op.ox + e.clientX - op.x;
        this.oy = op.oy + e.clientY - op.y;
        this.render();
        return;
      case 'move': {
        const d: Pt = [p[0] - op.p0[0], p[1] - op.p0[1]];
        // A click is no move: nothing moves (or snaps to the grid) until the pointer has travelled.
        if (!this.travelled(op, p)) return;
        const dd = this.moveSnap(op, d);
        this.replace(op.orig, (s) => transformShape(s, translate(dd[0], dd[1])));
        return;
      }
      case 'scale': {
        if (!this.travelled(op, p)) return;
        const q = this.snap(p, { exclude: new Set(op.orig.keys()) });
        const b = { ...op.box };
        const h = op.handle;
        if (h === 0 || h === 6 || h === 7) b.minX = q[0];
        if (h === 2 || h === 3 || h === 4) b.maxX = q[0];
        if (h === 0 || h === 1 || h === 2) b.minY = q[1];
        if (h === 4 || h === 5 || h === 6) b.maxY = q[1];
        if (e.shiftKey && h % 2 === 0) {
          // Corners keep the proportions.
          const k = Math.max((b.maxX - b.minX) / (op.box.maxX - op.box.minX), (b.maxY - b.minY) / (op.box.maxY - op.box.minY));
          const w = (op.box.maxX - op.box.minX) * k;
          const hh = (op.box.maxY - op.box.minY) * k;
          if (h === 0 || h === 6) b.minX = b.maxX - w;
          else b.maxX = b.minX + w;
          if (h === 0 || h === 2) b.minY = b.maxY - hh;
          else b.maxY = b.minY + hh;
        }
        if (Math.abs(b.maxX - b.minX) < 1e-6 || Math.abs(b.maxY - b.minY) < 1e-6) return;
        const m = boxToBox(op.box, b);
        this.replace(op.orig, (s) => transformShape(s, m));
        return;
      }
      case 'rotate': {
        if (!this.travelled(op, p)) return;
        const cx = (op.box.minX + op.box.maxX) / 2;
        const cy = (op.box.minY + op.box.maxY) / 2;
        let deg = ((Math.atan2(p[1] - cy, p[0] - cx) - Math.atan2(op.p0[1] - cy, op.p0[0] - cx)) * 180) / Math.PI;
        if (e.shiftKey) deg = Math.round(deg / 15) * 15;
        this.replace(op.orig, (s) => transformShape(s, rotation(deg, cx, cy)));
        host.status(`Döndürme: ${Math.round(deg)}°`);
        return;
      }
      case 'marquee':
        op.p1 = p;
        this.render();
        return;
      case 'draw': {
        const q = this.snap(p, { from: op.p0 });
        op.p1 = q;
        if (host.tool === 'pen') this.drawing.penDrag(q);
        this.render();
        return;
      }
    }
  }

  /** Whether a move, scale or rotation has begun: the pointer has travelled a few pixels since the press. */
  private travelled(op: { p0: Pt; moved: boolean }, p: Pt): boolean {
    if (!op.moved && Math.hypot(p[0] - op.p0[0], p[1] - op.p0[1]) * this.zoom < DRAG_PX) return false;
    op.moved = true;
    return true;
  }

  /**
   * The move's snapped offset: every snap source of the moving shapes (box
   * corners and centre, path nodes) is tried at its new place; the one
   * nearest a target wins. With none near, the box corner goes to the grid.
   */
  private moveSnap(op: Extract<Op, { kind: 'move' }>, d: Pt): Pt {
    const exclude = new Set(op.orig.keys());
    let best: { d: Pt; dist: number; hit: Snapper['hit'] } | null = null;
    for (const s of op.sources) {
      const at: Pt = [s[0] + d[0], s[1] + d[1]];
      const q = this.snap(at, { exclude, noGrid: true });
      const hit = this.snapper.hit;
      if (!hit) continue;
      if (!best || hit.d < best.dist) best = { d: [q[0] - s[0], q[1] - s[1]], dist: hit.d, hit };
    }
    if (best) {
      this.snapper.hit = best.hit;
      return best.d;
    }
    const corner = this.snap([op.box.minX + d[0], op.box.minY + d[1]], { exclude });
    return [corner[0] - op.box.minX, corner[1] - op.box.minY];
  }

  /**
   * Double clicks are counted here, not by the browser: the canvas captures
   * the pointer and re-renders on a press, so the element pressed is gone by
   * the release and the browser fires neither click nor dblclick.
   */
  private up(e: PointerEvent): void {
    const press = this.press;
    this.press = null;
    this.release(e);
    if (!press || press.travelled) {
      this.lastClick = null;
      return;
    }
    const last = this.lastClick;
    if (last && press.t - last.t <= DOUBLE_MS && Math.hypot(press.clientX - last.clientX, press.clientY - last.clientY) <= CLICK_SLOP_PX) {
      this.lastClick = null;
      this.dbl(press);
    } else this.lastClick = { t: press.t, clientX: press.clientX, clientY: press.clientY };
  }

  private release(e: PointerEvent): void {
    const op = this.op;
    this.op = null;
    const host = this.host;
    if (this.rulers.up()) return this.snapper.reset();
    if (host.tool === 'measure') {
      this.measure.up();
      return;
    }
    if (!op && host.nodeEdit && host.tool === 'node') {
      this.nodes.up();
      this.snapper.hit = null;
      this.render();
      return;
    }
    this.snapper.hit = null;
    if (!op) return;
    switch (op.kind) {
      case 'move':
        if (sameShapes(op.orig, host.doc)) {
          host.commit('');
          if (op.narrow) host.select(op.narrow);
        } else host.commit('Taşı');
        break;
      case 'scale':
        host.commit('Boyutlandır');
        break;
      case 'rotate':
        host.commit('Döndür');
        host.status('');
        break;
      case 'marquee': {
        const b: Box = { minX: Math.min(op.p0[0], op.p1[0]), minY: Math.min(op.p0[1], op.p1[1]), maxX: Math.max(op.p0[0], op.p1[0]), maxY: Math.max(op.p0[1], op.p1[1]) };
        if (b.maxX - b.minX > 1e-6 || b.maxY - b.minY > 1e-6) {
          const inside = host.doc.shapes.filter((s) => {
            if (s.hidden || s.locked) return false;
            const sb = shapeBox(s);
            return sb.minX >= b.minX && sb.maxX <= b.maxX && sb.minY >= b.minY && sb.maxY <= b.maxY;
          });
          const ids = new Set(op.add ? host.selection : []);
          for (const s of inside) for (const m of this.groupOf(s.id)) ids.add(m);
          host.select([...ids]);
        }
        this.render();
        break;
      }
      case 'draw': {
        if (host.tool === 'pen') {
          this.render();
          break;
        }
        this.drawing.place(op.p0, op.p1, e.altKey);
        this.render();
        break;
      }
      case 'pan':
        break;
    }
  }

  /** A double click: `at` is the second press, `target` what it pressed (by now perhaps re-rendered away; its attributes still read). */
  private dbl(at: { clientX: number; clientY: number; target: Element }): void {
    const host = this.host;
    const target = at.target;
    if (this.rulers.dbl(at, target)) return;
    if (host.tool === 'line' || host.tool === 'pen') {
      this.drawing.dropLast();
      return this.drawing.finish(false);
    }
    if (host.nodeEdit && host.tool === 'node' && this.nodes.dbl(this.toDoc(at), target)) return;
    const id = target.closest('[data-id]')?.getAttribute('data-id');
    const shape = id ? host.doc.shapes.find((s) => s.id === id) : undefined;
    if (shape?.kind === 'path' && !shape.locked) {
      host.select([shape.id]);
      host.editNodes(shape.id);
      host.status('Düğüm düzenleme: düğüme tık seçer (Shift ekler), boşlukta sürükleyince kutu içindekiler seçilir; parçaya çift tık düğüm ekler, düğüme çift tık köşe/yumuşak yapar. İşlemler sağda, Esc bitirir.');
    }
  }

  private wheel(e: WheelEvent): void {
    e.preventDefault();
    const r = this.svg.getBoundingClientRect();
    this.zoomBy(e.deltaY < 0 ? 1.15 : 1 / 1.15, [e.clientX - r.left, e.clientY - r.top]);
  }

  // ── Tools ────────────────────────────────────────────────────────────

  /** Ends the polyline or pen path being drawn (Enter). */
  finishDraft(closed: boolean): void {
    this.drawing.finish(closed);
  }

  /** Esc on the canvas: whatever is half done goes first. */
  cancel(): boolean {
    if (this.picking) {
      this.picking = null;
      this.host.status('Nokta seçmekten vazgeçildi.');
      this.render();
      return true;
    }
    this.rulers.closePopup();
    if (this.host.tool === 'measure' && this.measure.cancel()) return true;
    if (this.host.nodeEdit && this.nodes.cancel()) return true;
    return this.cancelDraft();
  }

  cancelDraft(): boolean {
    return this.drawing.cancel();
  }

  /** The tool changed: its half-done work goes. */
  toolChanged(): void {
    this.cancelDraft();
    this.measure.reset();
    if (this.nodes.mode) this.nodes.setMode(null);
  }

  private replace(orig: Map<string, SvgShape>, fn: (s: SvgShape) => SvgShape): void {
    const doc = this.host.doc;
    doc.shapes = doc.shapes.map((s) => (orig.has(s.id) ? fn(orig.get(s.id)!) : s));
    this.host.changed();
  }

  private groupOf(id: string): string[] {
    const s = this.host.doc.shapes.find((x) => x.id === id);
    if (!s?.group) return [id];
    return this.host.doc.shapes.filter((x) => x.group === s.group && !x.locked).map((x) => x.id);
  }
}

/** Where a moving selection snaps from: its box corners and centre, and the nodes of its paths (at most 64). */
function moveSources(sel: readonly SvgShape[], b: Box): Pt[] {
  const out: Pt[] = [
    [b.minX, b.minY],
    [b.maxX, b.minY],
    [b.minX, b.maxY],
    [b.maxX, b.maxY],
    [(b.minX + b.maxX) / 2, (b.minY + b.maxY) / 2],
  ];
  for (const s of sel) if (s.kind === 'path') for (const sp of s.subs) for (const n of sp.nodes) if (out.length < 69) out.push([n.x, n.y]);
  return out;
}

function sameShapes(orig: Map<string, SvgShape>, doc: SvgDoc): boolean {
  return doc.shapes.every((s) => !orig.has(s.id) || JSON.stringify(s) === JSON.stringify(orig.get(s.id)));
}
