import { listen, type Disposable } from '../../../core/disposable';
import type { ProcessingModel } from '../../../processing/model';
import { stepName } from '../../../processing/model';
import { INPUT_TYPES } from '../../../processing/modelEdit';
import type { ProcessingTool } from '../../../processing/types';
import { h } from '../../dom';
import { icon } from '../../icons';
import {
  boxesBounds,
  CANVAS,
  curve as edgeCurve,
  DESIGNER_TEXTS,
  edgeLabels,
  fitView,
  inputPort,
  snap,
  stepEntry,
  stepMeta,
  stepPort,
  zoomAt,
  zoomFloor,
  type NodeRef,
  type Pt,
  type View,
} from './designerPlan';

/**
 * The flow diagram of the model designer. Boxes are HTML (text, icons and
 * focus behave like the rest of the UI), connections an SVG layer under
 * them; both sit in one transformed "world" so panning and zooming move
 * them together. The canvas only reports gestures; the designer changes
 * the model and calls `render` again.
 */

export type { NodeRef };
const C = DESIGNER_TEXTS.canvas;
const { inputW: INPUT_W, inputH: INPUT_H, stepW: STEP_W, stepH: STEP_H, grid: GRID } = CANVAS;

export interface CanvasEvents {
  select(ref: NodeRef | null): void;
  /** A box is being dragged; `done` on release (the designer records one undo step). */
  move(ref: NodeRef, at: Pt, done: boolean): void;
  /** An output port was dropped on a step box. */
  connect(from: NodeRef, toStep: string, client: Pt): void;
  /** Double click on a box: go to its settings. */
  open(ref: NodeRef): void;
}

const sameRef = (a: NodeRef | null, b: NodeRef | null) => !!a && !!b && a.kind === b.kind && (a.kind === 'input' ? a.name === (b as { name: string }).name : a.id === (b as { id: string }).id);
const SVG = 'http://www.w3.org/2000/svg';

export class ModelCanvas {
  readonly el: HTMLElement;
  private readonly world: HTMLElement;
  private readonly edges: SVGSVGElement;
  private readonly events: CanvasEvents;
  private readonly lookup: (id: string) => ProcessingTool | undefined;
  private view: View = { x: 0, y: 0, k: 1 };
  /** Zooming's lowest scale: the last fit's when that was below `zoomMin` (designerPlan.ts `zoomFloor`). */
  private floor: number = CANVAS.zoomMin;
  private model: ProcessingModel | null = null;
  private selected: NodeRef | null = null;
  private problems = new Map<string, string>();
  private wire: SVGPathElement | null = null;
  private readonly subs: Disposable[] = [];

  constructor(events: CanvasEvents, lookup: (id: string) => ProcessingTool | undefined) {
    this.events = events;
    this.lookup = lookup;
    this.edges = document.createElementNS(SVG, 'svg');
    this.edges.classList.add('mcanvas__edges');
    this.world = h('div', { class: 'mcanvas__world' }, this.edges);
    const zoom = (f: number) => {
      const r = this.el.getBoundingClientRect();
      this.zoomAt({ x: r.width / 2, y: r.height / 2 }, f);
    };
    const tools = h(
      'div',
      { class: 'mcanvas__tools' },
      this.toolButton('zoomOut', C.zoomOut, () => zoom(1 / CANVAS.zoomStep)),
      this.toolButton('zoomIn', C.zoomIn, () => zoom(CANVAS.zoomStep)),
      this.toolButton('zoomExtents', C.fit, () => this.fit()),
    );
    this.el = h('div', { class: 'mcanvas', tabindex: '0', 'aria-label': C.label }, this.world, tools);
    this.subs.push(
      listen<PointerEvent>(this.el, 'pointerdown', (e) => this.onDown(e)),
      listen<WheelEvent>(this.el, 'wheel', (e) => this.onWheel(e), { passive: false }),
      listen<MouseEvent>(this.el, 'dblclick', (e) => {
        const node = (e.target as HTMLElement).closest<HTMLElement>('.mnode');
        if (node) this.events.open(this.refOf(node));
        else if (!(e.target as HTMLElement).closest('.mcanvas__tools')) this.fit();
      }),
    );
  }

  private toolButton(name: string, label: string, run: () => void): HTMLButtonElement {
    const b = h('button', { class: 'ibtn', type: 'button', 'aria-label': label, title: label }, icon(name, 16));
    b.addEventListener('click', run);
    b.addEventListener('pointerdown', (e) => e.stopPropagation());
    return b;
  }

  dispose(): void {
    this.subs.forEach((d) => d());
  }

  /** World point under a screen point (drop position of a tool from the palette). */
  worldAt(client: Pt): Pt | null {
    const r = this.el.getBoundingClientRect();
    if (client.x < r.left || client.x > r.right || client.y < r.top || client.y > r.bottom) return null;
    return { x: (client.x - r.left - this.view.x) / this.view.k, y: (client.y - r.top - this.view.y) / this.view.k };
  }

  render(model: ProcessingModel, selected: NodeRef | null, problems: Map<string, string>): void {
    this.model = model;
    this.selected = selected;
    this.problems = problems;
    this.world.replaceChildren(this.edges, ...model.inputs.map((i) => this.inputBox(i.name)), ...model.steps.map((s) => this.stepBox(s.id)));
    this.drawEdges();
    this.applyView();
  }

  /** Frames every box; zooms out only (never above 1:1). */
  fit(): void {
    const r = this.el.getBoundingClientRect();
    this.view = fitView(this.model && boxesBounds(this.model), r.width, r.height);
    this.floor = zoomFloor(this.view.k);
    this.applyView();
  }

  private applyView(): void {
    this.world.style.transform = `translate(${this.view.x}px, ${this.view.y}px) scale(${this.view.k})`;
    this.el.style.setProperty('--mgrid', `${GRID * 2 * this.view.k}px`);
    this.el.style.backgroundPosition = `${this.view.x}px ${this.view.y}px`;
  }

  private zoomAt(p: Pt, f: number): void {
    this.view = zoomAt(this.view, p, f, this.floor);
    this.applyView();
  }

  private inputPos(name: string): Pt {
    return this.model?.inputPositions?.[name] ?? { x: 40, y: 40 };
  }

  private inputBox(name: string): HTMLElement {
    const def = this.model!.inputs.find((i) => i.name === name)!;
    const type = INPUT_TYPES.find((t) => t.type === def.type);
    const p = this.inputPos(name);
    const ref: NodeRef = { kind: 'input', name };
    return h(
      'div',
      {
        class: 'mnode mnode--input',
        style: `left:${p.x}px;top:${p.y}px;width:${INPUT_W}px;height:${INPUT_H}px`,
        dataset: { kind: 'input', ref: name },
        'aria-selected': String(sameRef(ref, this.selected)),
        role: 'button',
        tabindex: '-1',
      },
      h('span', { class: 'mnode__icon' }, icon(type?.icon ?? 'processing', 16)),
      h('span', { class: 'mnode__text' }, h('span', { class: 'mnode__name' }, def.label), h('span', { class: 'mnode__meta' }, C.inputMeta(type?.label ?? def.type, !!def.optional))),
      h('span', { class: 'mnode__port', title: C.port }),
    );
  }

  private stepBox(id: string): HTMLElement {
    const s = this.model!.steps.find((x) => x.id === id)!;
    const tool = this.lookup(s.tool);
    const p = s.position ?? { x: 0, y: 0 };
    const ref: NodeRef = { kind: 'step', id };
    const problem = this.problems.get(id);
    const meta = stepMeta(s, tool, problem);
    return h(
      'div',
      {
        class: 'mnode mnode--step',
        style: `left:${p.x}px;top:${p.y}px;width:${STEP_W}px;height:${STEP_H}px`,
        dataset: { kind: 'step', ref: id },
        'aria-selected': String(sameRef(ref, this.selected)),
        'data-problem': problem ? '' : null,
        title: problem ?? '',
        role: 'button',
        tabindex: '-1',
      },
      h('span', { class: 'mnode__in' }),
      h('span', { class: 'mnode__icon' }, icon(tool?.icon ?? 'processing', 16)),
      h(
        'span',
        { class: 'mnode__text' },
        h('span', { class: 'mnode__name' }, stepName(s, this.lookup)),
        h('span', { class: 'mnode__meta' }, 'warn' in meta ? h('span', { class: 'mnode__warn' }, icon('warning', 12), meta.warn) : meta.text),
      ),
      tool?.outputs?.length ? h('span', { class: 'mnode__port', title: C.port }) : null,
    );
  }

  /** Where an edge starts (a box's port) and ends (a step's left side), in world units. */
  private portOf(ref: NodeRef): Pt | null {
    if (ref.kind === 'input') return inputPort(this.inputPos(ref.name));
    const s = this.model!.steps.find((x) => x.id === ref.id);
    return s ? stepPort(s.position ?? { x: 0, y: 0 }) : null;
  }

  private drawEdges(): void {
    const m = this.model!;
    this.edges.replaceChildren();
    // The curves first, then every label over them, at its target step (designerPlan.ts `edgeLabels`).
    const texts: SVGTextElement[] = [];
    for (const e of edgeLabels(m, this.lookup)) {
      const a = this.portOf(e.from);
      const target = m.steps.find((s) => s.id === e.to);
      if (!a || !target) continue;
      const b = stepEntry(target.position ?? { x: 0, y: 0 });
      const path = document.createElementNS(SVG, 'path');
      path.setAttribute('d', curve(a, b));
      path.classList.add('medge');
      if (sameRef(e.from, this.selected) || sameRef({ kind: 'step', id: e.to }, this.selected)) path.classList.add('medge--on');
      const title = document.createElementNS(SVG, 'title');
      title.textContent = e.title;
      path.append(title);
      this.edges.append(path);
      const text = document.createElementNS(SVG, 'text');
      text.classList.add('medge__label');
      text.setAttribute('x', String(e.at.x));
      text.setAttribute('y', String(e.at.y));
      text.setAttribute('text-anchor', 'end');
      text.textContent = e.text;
      texts.push(text);
    }
    this.edges.append(...texts);
  }

  private refOf(node: HTMLElement): NodeRef {
    return node.dataset.kind === 'input' ? { kind: 'input', name: node.dataset.ref! } : { kind: 'step', id: node.dataset.ref! };
  }

  private onWheel(e: WheelEvent): void {
    e.preventDefault();
    const r = this.el.getBoundingClientRect();
    this.zoomAt({ x: e.clientX - r.left, y: e.clientY - r.top }, Math.exp(-e.deltaY * CANVAS.wheel));
  }

  private onDown(e: PointerEvent): void {
    if (e.button !== 0 || (e.target as HTMLElement).closest('.mcanvas__tools')) return;
    const target = e.target as HTMLElement;
    const node = target.closest<HTMLElement>('.mnode');
    this.el.focus({ preventScroll: true });
    const start = { x: e.clientX, y: e.clientY };
    let moved = false;
    const track = (onMove: (dx: number, dy: number, ev: PointerEvent) => void, onUp: (ev: PointerEvent) => void) => {
      const move = (ev: PointerEvent) => {
        const dx = ev.clientX - start.x;
        const dy = ev.clientY - start.y;
        if (!moved && Math.hypot(dx, dy) < CANVAS.drag) return;
        moved = true;
        onMove(dx, dy, ev);
      };
      const up = (ev: PointerEvent) => {
        window.removeEventListener('pointermove', move);
        window.removeEventListener('pointerup', up);
        onUp(ev);
      };
      window.addEventListener('pointermove', move);
      window.addEventListener('pointerup', up);
    };

    if (node && target.classList.contains('mnode__port')) {
      // Connect: a wire follows the pointer; dropping it on a step asks which input it feeds.
      e.preventDefault();
      const from = this.refOf(node);
      const a = this.portOf(from)!;
      this.wire = document.createElementNS(SVG, 'path');
      this.wire.classList.add('medge', 'medge--wire');
      this.edges.append(this.wire);
      const r = this.el.getBoundingClientRect();
      track(
        (_dx, _dy, ev) => {
          const b = { x: (ev.clientX - r.left - this.view.x) / this.view.k, y: (ev.clientY - r.top - this.view.y) / this.view.k };
          this.wire?.setAttribute('d', curve(a, b));
          const over = document.elementFromPoint(ev.clientX, ev.clientY)?.closest<HTMLElement>('.mnode--step');
          this.world.querySelectorAll('.mnode[data-drop]').forEach((n) => n.removeAttribute('data-drop'));
          if (over && over !== node) over.setAttribute('data-drop', '');
        },
        (ev) => {
          this.wire?.remove();
          this.wire = null;
          this.world.querySelectorAll('.mnode[data-drop]').forEach((n) => n.removeAttribute('data-drop'));
          const over = document.elementFromPoint(ev.clientX, ev.clientY)?.closest<HTMLElement>('.mnode--step');
          if (moved && over && over !== node) this.events.connect(from, over.dataset.ref!, { x: ev.clientX, y: ev.clientY });
        },
      );
      return;
    }

    if (node) {
      // Select and drag a box (snapped to the grid).
      const ref = this.refOf(node);
      if (!sameRef(ref, this.selected)) this.events.select(ref);
      const origin = ref.kind === 'input' ? this.inputPos(ref.name) : (this.model!.steps.find((s) => s.id === ref.id)?.position ?? { x: 0, y: 0 });
      const at = (dx: number, dy: number) => ({ x: snap(origin.x + dx / this.view.k), y: snap(origin.y + dy / this.view.k) });
      let last = origin;
      track(
        (dx, dy) => {
          last = at(dx, dy);
          this.events.move(ref, last, false);
        },
        () => {
          if (moved) this.events.move(ref, last, true);
        },
      );
      return;
    }

    // Background: pan; a click without moving selects the model itself.
    const from = { ...this.view };
    this.el.setAttribute('data-panning', '');
    track(
      (dx, dy) => {
        this.view = { ...from, x: from.x + dx, y: from.y + dy };
        this.applyView();
      },
      () => {
        this.el.removeAttribute('data-panning');
        if (!moved) this.events.select(null);
      },
    );
  }
}

/** An edge as an SVG path (designerPlan.ts `curve`). */
function curve(a: Pt, b: Pt): string {
  const { c1, c2 } = edgeCurve(a, b);
  return `M ${a.x} ${a.y} C ${c1.x} ${c1.y}, ${c2.x} ${c2.y}, ${b.x} ${b.y}`;
}
