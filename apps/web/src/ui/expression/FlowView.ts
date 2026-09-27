import { listen, type Disposable } from '../../core/disposable';
import type { Flow, FlowNode, FlowType } from '../../model/expression/flow';
import { h } from '../dom';
import { icon } from '../icons';

/**
 * The expression's flow (DESIGN.md §7.16, docs/adr/0101): the nodes the core
 * laid out, as HTML boxes over an SVG layer of edges in one transformed
 * world (the model designer's canvas does the same), so text, focus and
 * the theme behave as in the rest of the dialog. A value goes from a node's
 * output (right) into another node's input (left); the result stands on
 * the right. The view only reports gestures; the builder changes the text
 * through the core and draws the new flow.
 *
 * - drag from an output to an input: connect (what the input held goes apart);
 * - drag a connected input's pin away: take the connection off (drop it on
 *   another input to move it there);
 * - drag from an empty input to a node: connect that node into it;
 * - a node not connected to the result is moved by its title;
 * - the wheel zooms, dragging the background pans, a double click on the
 *   background shows everything; Delete removes the selected node.
 */

type Pt = { x: number; y: number };
const SVG = 'http://www.w3.org/2000/svg';

/** What a palette entry carries when dragged onto the flow. */
export const FLOW_DRAG_TYPE = 'application/x-kentos-expr';

const TYPE_NAME: Record<FlowType, string> = { any: 'değer', number: 'sayı', text: 'metin', bool: 'koşul' };

export interface FlowViewEvents {
  select(id: string | null): void;
  connect(from: string, to: string, port: number): void;
  disconnect(to: string, port: number): void;
  /** A tree not connected to the result, moved to a world point. */
  move(tree: number, at: [number, number]): void;
  remove(id: string): void;
  addPort(id: string): void;
  removePort(id: string, port: number): void;
  /** A palette entry dropped: its top-left at a world point, into an input when `to` is given. */
  drop(key: string, at: [number, number], to?: { readonly node: string; readonly port: number }): void;
  /** Double click on a node: its value to edit. */
  open(id: string): void;
  undo(): void;
  redo(): void;
}

/** The tree a node is in (-1 for the result). */
const treeOf = (id: string) => (id === 'r' ? -1 : Number(id.split('.')[0]));
const isRoot = (id: string) => id !== 'r' && !id.includes('.');

export class FlowView {
  readonly el: HTMLElement;
  private readonly world: HTMLElement;
  private readonly edges: SVGSVGElement;
  private readonly note: HTMLElement;
  private readonly events: FlowViewEvents;
  private view = { x: 0, y: 0, k: 1 };
  private flow: Flow | null = null;
  private selected: string | null = null;
  private values: ReadonlyMap<string, string> = new Map();
  private fitted = false;
  private wire: SVGPathElement | null = null;
  /** A tree being moved and by how much (the edges follow it). */
  private moving: { readonly tree: number; dx: number; dy: number } | null = null;
  private readonly subs: Disposable[] = [];

  constructor(events: FlowViewEvents) {
    this.events = events;
    this.edges = document.createElementNS(SVG, 'svg');
    this.edges.classList.add('xflow__edges');
    this.world = h('div', { class: 'xflow__world' }, this.edges);
    this.note = h('div', { class: 'xflow__note', hidden: true });
    const zoom = (f: number) => {
      const r = this.el.getBoundingClientRect();
      this.zoomAt({ x: r.width / 2, y: r.height / 2 }, f);
    };
    const tools = h(
      'div',
      { class: 'xflow__tools' },
      this.toolButton('zoomOut', 'Uzaklaş', () => zoom(1 / 1.25)),
      this.toolButton('zoomIn', 'Yakınlaş', () => zoom(1.25)),
      this.toolButton('zoomExtents', 'Tümünü göster (arka plana çift tık)', () => this.fit()),
    );
    this.el = h('div', { class: 'xflow', tabindex: '0', role: 'application', 'aria-label': 'İfadenin akışı' }, this.world, this.note, tools);
    this.subs.push(
      listen<PointerEvent>(this.el, 'pointerdown', (e) => this.onDown(e)),
      listen<WheelEvent>(this.el, 'wheel', (e) => this.onWheel(e), { passive: false }),
      listen<MouseEvent>(this.el, 'dblclick', (e) => {
        const target = e.target as HTMLElement;
        if (target.closest('.xflow__tools')) return;
        const node = target.closest<HTMLElement>('.xfn');
        if (node) this.events.open(node.dataset.id!);
        else this.fit();
      }),
      listen<KeyboardEvent>(this.el, 'keydown', (e) => this.onKey(e)),
      listen<DragEvent>(this.el, 'dragover', (e) => this.onDragOver(e)),
      listen<DragEvent>(this.el, 'dragleave', () => this.mark(null)),
      listen<DragEvent>(this.el, 'drop', (e) => this.onDrop(e)),
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

  /** Draws a flow; `values`: each node's value on the previewed object, by id. */
  render(flow: Flow, selected: string | null, values: ReadonlyMap<string, string>): void {
    this.flow = flow;
    this.selected = selected;
    this.values = values;
    this.world.replaceChildren(this.edges, ...flow.nodes.map((n) => this.box(n)));
    this.drawEdges();
    this.showNote();
    if (!this.fitted) {
      // The first time it is shown: everything in view (after the layout gives the canvas a size).
      requestAnimationFrame(() => {
        if (this.el.getBoundingClientRect().width > 0 && !this.fitted) {
          this.fitted = true;
          this.fit();
        }
      });
    }
    this.applyView();
  }

  /** Frames everything again on the next drawing (the view was shown anew). */
  refit(): void {
    this.fitted = false;
  }

  /** The previewed values only (another object): the boxes stay. */
  setValues(values: ReadonlyMap<string, string>): void {
    this.values = values;
    this.world.querySelectorAll<HTMLElement>('.xfn').forEach((el) => {
      const v = el.querySelector('.xfn__value');
      if (v) this.fillValue(v as HTMLElement, el.dataset.id!);
    });
  }

  /** Frames every node; not larger than the dialog's size. */
  fit(): void {
    const r = this.el.getBoundingClientRect();
    const f = this.flow;
    if (!f || !r.width) return;
    const [l, t, rt, b] = f.bounds;
    const pad = 36;
    const w = Math.max(1, rt - l);
    const hh = Math.max(1, b - t);
    const k = Math.max(0.3, Math.min(1, (r.width - pad * 2) / w, (r.height - pad * 2) / hh));
    this.view = { k, x: (r.width - w * k) / 2 - l * k, y: (r.height - hh * k) / 2 - t * k };
    this.applyView();
  }

  /** The world point under a screen point. */
  worldAt(client: Pt): Pt {
    const r = this.el.getBoundingClientRect();
    return { x: (client.x - r.left - this.view.x) / this.view.k, y: (client.y - r.top - this.view.y) / this.view.k };
  }

  /** The world point at the middle of the view (where a new node goes when nothing says where). */
  centre(): Pt {
    const r = this.el.getBoundingClientRect();
    return this.worldAt({ x: r.left + r.width / 2, y: r.top + r.height / 2 });
  }

  focus(): void {
    this.el.focus({ preventScroll: true });
  }

  private showNote(): void {
    const f = this.flow!;
    if (f.error) {
      this.note.hidden = false;
      this.note.className = 'xflow__note xflow__note--error';
      this.note.replaceChildren(icon('error', 16), h('span', null, `İfade okunamıyor: ${f.error.text} Akışta göstermek için Metin'de düzeltin.`));
    } else if (f.nodes.length === 1) {
      this.note.hidden = false;
      this.note.className = 'xflow__note';
      this.note.replaceChildren(icon('info', 16), h('span', null, 'Ağaçtan bir öğeyi buraya sürükleyin ya da çift tıklayın; değerini Sonuç’un girişine bağlayın.'));
    } else {
      this.note.hidden = true;
    }
  }

  private box(n: FlowNode): HTMLElement {
    const f = this.flow!;
    const apart = n.id !== 'r' && treeOf(n.id) > 0;
    const status = n.error ? 'error' : n.warnings.length ? 'warning' : null;
    const title = [n.error, ...n.warnings].filter(Boolean).join('\n');
    const head = h(
      'div',
      { class: 'xfn__head', style: `height:${f.head}px`, title: apart && isRoot(n.id) ? 'Taşımak için sürükleyin' : null },
      h('span', { class: 'xfn__title', title: n.title }, n.title),
      status ? h('span', { class: `xfn__status xfn__status--${status}`, title }, icon(status, 14)) : null,
      n.grows ? this.nodeButton('plus', 'Giriş ekle', () => this.events.addPort(n.id)) : null,
    );
    const ports = n.ports.map((p, k) => {
      // The type when it says something the name does not (not for “any value”).
      const tag = p.note
        ? h('span', { class: 'xfn__note', title: p.note }, icon('info', 12))
        : p.type !== 'any' && p.name !== TYPE_NAME[p.type]
          ? h('span', { class: 'xfn__ptype', 'data-type': p.type }, TYPE_NAME[p.type])
          : null;
      return h(
        'div',
        {
          class: 'xfn__port',
          style: `top:${p.y - f.row / 2}px;height:${f.row}px`,
          dataset: { port: String(k), type: p.type },
          'data-on': p.from ? '' : null,
          'data-optional': p.optional ? '' : null,
          title: `${p.name}: ${TYPE_NAME[p.type]}${p.optional ? ' (isteğe bağlı)' : ''}${p.note ? `\n${p.note}` : ''}`,
        },
        h('span', { class: 'xfn__pin', title: p.from ? 'Bağlantıyı ayırmak ya da taşımak için sürükleyin' : 'Bir düğüme sürükleyip bağlayın' }),
        h('span', { class: 'xfn__pname' }, p.name),
        p.removable ? this.nodeButton('close', 'Bu girişi kaldır', () => this.events.removePort(n.id, k)) : tag,
      );
    });
    // A constant is its title: no row for its value.
    const room = n.h - f.head - n.ports.length * f.row >= f.value;
    const value = room ? h('div', { class: 'xfn__value', style: `height:${f.value}px` }) : null;
    if (value) this.fillValue(value, n.id);
    return h(
      'div',
      {
        class: `xfn xfn--${n.kind}${apart ? ' xfn--apart' : ''}`,
        style: `left:${n.x}px;top:${n.y}px;width:${n.w}px;height:${n.h}px`,
        dataset: { id: n.id, kind: n.kind },
        'aria-selected': String(n.id === this.selected),
        'data-status': status,
        role: 'button',
        tabindex: '-1',
        'aria-label': `${n.title}${n.error ? `: ${n.error}` : ''}`,
      },
      head,
      ...ports,
      value,
      n.kind === 'result' ? null : h('span', { class: 'xfn__out', dataset: { type: n.type }, style: `top:${f.head / 2}px`, title: 'Bir girişe sürükleyip bağlayın' }),
    );
  }

  private fillValue(el: HTMLElement, id: string): void {
    const v = this.values.get(id);
    el.textContent = v === undefined ? (id === 'r' ? '—' : '') : id === 'r' ? v : `= ${v}`;
    el.title = el.textContent;
  }

  private nodeButton(name: string, label: string, run: () => void): HTMLButtonElement {
    const b = h('button', { class: 'xfn__btn', type: 'button', 'aria-label': label, title: label }, icon(name, 12));
    b.addEventListener('pointerdown', (e) => e.stopPropagation());
    b.addEventListener('click', (e) => {
      e.stopPropagation();
      run();
    });
    return b;
  }

  private node(id: string): FlowNode | undefined {
    return this.flow?.nodes.find((n) => n.id === id);
  }

  /** Where a node stands now: its place, or a tree being moved. */
  private at(n: FlowNode): Pt {
    const m = this.moving;
    return m && treeOf(n.id) === m.tree ? { x: n.x + m.dx, y: n.y + m.dy } : { x: n.x, y: n.y };
  }

  /** A node's output point. */
  private outOf(n: FlowNode): Pt {
    const p = this.at(n);
    return { x: p.x + n.w, y: p.y + this.flow!.head / 2 };
  }

  private drawEdges(skip?: { readonly to: string; readonly port: number }): void {
    const f = this.flow!;
    this.edges.replaceChildren();
    for (const n of f.nodes) {
      n.ports.forEach((p, k) => {
        if (!p.from || (skip && skip.to === n.id && skip.port === k)) return;
        const c = this.node(p.from);
        if (!c) return;
        const path = document.createElementNS(SVG, 'path');
        const to = this.at(n);
        path.setAttribute('d', curve(this.outOf(c), { x: to.x, y: to.y + p.y }));
        path.classList.add('xflow__edge');
        path.dataset.type = c.type;
        if (this.selected && (this.selected === n.id || this.selected === c.id)) path.classList.add('xflow__edge--on');
        this.edges.append(path);
      });
    }
  }

  private applyView(): void {
    this.world.style.transform = `translate(${this.view.x}px, ${this.view.y}px) scale(${this.view.k})`;
    this.el.style.setProperty('--xgrid', `${20 * this.view.k}px`);
    this.el.style.backgroundPosition = `${this.view.x}px ${this.view.y}px`;
  }

  private zoomAt(p: Pt, f: number): void {
    const k = Math.min(2, Math.max(0.3, this.view.k * f));
    const wx = (p.x - this.view.x) / this.view.k;
    const wy = (p.y - this.view.y) / this.view.k;
    this.view = { k, x: p.x - wx * k, y: p.y - wy * k };
    this.applyView();
  }

  private onWheel(e: WheelEvent): void {
    e.preventDefault();
    const r = this.el.getBoundingClientRect();
    this.zoomAt({ x: e.clientX - r.left, y: e.clientY - r.top }, Math.exp(-e.deltaY * 0.0015));
  }

  private onKey(e: KeyboardEvent): void {
    const mod = e.ctrlKey || e.metaKey;
    if (mod && (e.key === 'z' || e.key === 'Z')) {
      if (e.shiftKey) this.events.redo();
      else this.events.undo();
    } else if (mod && (e.key === 'y' || e.key === 'Y')) {
      this.events.redo();
    } else if ((e.key === 'Delete' || e.key === 'Backspace') && this.selected && this.selected !== 'r') {
      this.events.remove(this.selected);
    } else if (e.key === 'Escape' && this.selected) {
      this.events.select(null);
    } else {
      return;
    }
    e.preventDefault();
    e.stopPropagation();
  }

  /** The input under a screen point, if any. */
  private portAt(client: Pt): { node: string; port: number; el: HTMLElement } | null {
    const el = document.elementFromPoint(client.x, client.y)?.closest<HTMLElement>('.xfn__port');
    const node = el?.closest<HTMLElement>('.xfn');
    return el && node ? { node: node.dataset.id!, port: Number(el.dataset.port), el } : null;
  }

  /** Marks the input a drag would go into. */
  private mark(el: HTMLElement | null): void {
    this.world.querySelectorAll('.xfn__port[data-drop]').forEach((p) => p.removeAttribute('data-drop'));
    el?.setAttribute('data-drop', '');
  }

  private onDragOver(e: DragEvent): void {
    if (!e.dataTransfer?.types.includes(FLOW_DRAG_TYPE)) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = 'copy';
    this.mark(this.portAt({ x: e.clientX, y: e.clientY })?.el ?? null);
  }

  private onDrop(e: DragEvent): void {
    const key = e.dataTransfer?.getData(FLOW_DRAG_TYPE);
    this.mark(null);
    if (!key) return;
    e.preventDefault();
    const w = this.worldAt({ x: e.clientX, y: e.clientY });
    const port = this.portAt({ x: e.clientX, y: e.clientY });
    const width = this.flow?.nodes[0]?.w ?? 176;
    this.events.drop(key, [Math.round(w.x - width / 2), Math.round(w.y - (this.flow?.head ?? 30) / 2)], port ? { node: port.node, port: port.port } : undefined);
    this.focus();
  }

  private onDown(e: PointerEvent): void {
    if (e.button !== 0 || (e.target as HTMLElement).closest('.xflow__tools, .xfn__btn')) return;
    const target = e.target as HTMLElement;
    const box = target.closest<HTMLElement>('.xfn');
    this.focus();
    const start = { x: e.clientX, y: e.clientY };
    let moved = false;
    const track = (onMove: (dx: number, dy: number, ev: PointerEvent) => void, onUp: (ev: PointerEvent) => void) => {
      const move = (ev: PointerEvent) => {
        const dx = ev.clientX - start.x;
        const dy = ev.clientY - start.y;
        if (!moved && Math.hypot(dx, dy) < 4) return;
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
    const wireTo = (a: Pt, reversed: boolean) => {
      this.wire = document.createElementNS(SVG, 'path');
      this.wire.classList.add('xflow__edge', 'xflow__edge--wire');
      this.edges.append(this.wire);
      return (ev: PointerEvent) => {
        const b = this.worldAt({ x: ev.clientX, y: ev.clientY });
        this.wire?.setAttribute('d', reversed ? curve(b, a) : curve(a, b));
      };
    };
    const endWire = () => {
      this.wire?.remove();
      this.wire = null;
      this.mark(null);
    };

    const id = box?.dataset.id;
    const node = id ? this.node(id) : undefined;

    // From an output: into an input (or a node's first empty input).
    if (node && target.classList.contains('xfn__out')) {
      e.preventDefault();
      const draw = wireTo(this.outOf(node), false);
      track(
        (_dx, _dy, ev) => {
          draw(ev);
          this.mark(this.portAt({ x: ev.clientX, y: ev.clientY })?.el ?? null);
        },
        (ev) => {
          endWire();
          if (!moved) return;
          const into = this.inputAt({ x: ev.clientX, y: ev.clientY });
          if (into && into.node !== node.id) this.events.connect(node.id, into.node, into.port);
        },
      );
      return;
    }

    // From an input's pin: a connected one is taken off (and maybe moved), an empty one asks for a node.
    const portEl = target.closest<HTMLElement>('.xfn__port');
    if (node && portEl && target.classList.contains('xfn__pin')) {
      e.preventDefault();
      const port = Number(portEl.dataset.port);
      const p = node.ports[port];
      const pin = { x: node.x, y: node.y + p.y };
      if (p.from) {
        const child = this.node(p.from);
        if (!child) return;
        const draw = wireTo(this.outOf(child), false);
        track(
          (_dx, _dy, ev) => {
            this.drawEdges({ to: node.id, port });
            this.edges.append(this.wire!);
            draw(ev);
            this.mark(this.portAt({ x: ev.clientX, y: ev.clientY })?.el ?? null);
          },
          (ev) => {
            endWire();
            this.drawEdges();
            if (!moved) return;
            const into = this.portAt({ x: ev.clientX, y: ev.clientY });
            if (into && !(into.node === node.id && into.port === port)) this.events.connect(child.id, into.node, into.port);
            else if (!into) this.events.disconnect(node.id, port);
          },
        );
      } else {
        const draw = wireTo(pin, true);
        track(
          (_dx, _dy, ev) => draw(ev),
          (ev) => {
            endWire();
            if (!moved) return;
            const over = document.elementFromPoint(ev.clientX, ev.clientY)?.closest<HTMLElement>('.xfn');
            const from = over?.dataset.id;
            if (from && from !== 'r' && from !== node.id) this.events.connect(from, node.id, port);
          },
        );
      }
      return;
    }

    if (node) {
      if (node.id !== this.selected) this.events.select(node.id);
      // A tree not connected to the result moves by its root's title.
      if (isRoot(node.id) && treeOf(node.id) > 0 && target.closest('.xfn__head')) {
        const tree = treeOf(node.id);
        const origin = { x: node.x, y: node.y };
        const el = box!;
        let at = origin;
        track(
          (dx, dy) => {
            at = { x: Math.round(origin.x + dx / this.view.k), y: Math.round(origin.y + dy / this.view.k) };
            // The whole tree follows, and its edges.
            this.moving = { tree, dx: at.x - origin.x, dy: at.y - origin.y };
            this.world.querySelectorAll<HTMLElement>('.xfn').forEach((b) => {
              if (treeOf(b.dataset.id!) !== tree) return;
              const p = this.at(this.node(b.dataset.id!)!);
              b.style.left = `${p.x}px`;
              b.style.top = `${p.y}px`;
            });
            this.drawEdges();
            el.setAttribute('data-moving', '');
          },
          () => {
            el.removeAttribute('data-moving');
            this.moving = null;
            if (moved) this.events.move(tree, [at.x, at.y]);
          },
        );
      }
      return;
    }

    // The background: pan; a click without moving takes the selection off.
    const from = { ...this.view };
    this.el.setAttribute('data-panning', '');
    track(
      (dx, dy) => {
        this.view = { ...from, x: from.x + dx, y: from.y + dy };
        this.applyView();
      },
      () => {
        this.el.removeAttribute('data-panning');
        if (!moved && this.selected) this.events.select(null);
      },
    );
  }

  /** An input under a screen point: a port, or a node's first empty input. */
  private inputAt(client: Pt): { node: string; port: number } | null {
    const port = this.portAt(client);
    if (port) return { node: port.node, port: port.port };
    const box = document.elementFromPoint(client.x, client.y)?.closest<HTMLElement>('.xfn');
    const n = box ? this.node(box.dataset.id!) : undefined;
    if (!n) return null;
    const k = n.ports.findIndex((p) => !p.from);
    return k >= 0 ? { node: n.id, port: k } : null;
  }
}

const bend = (a: Pt, b: Pt) => Math.max(36, Math.abs(b.x - a.x) / 2);

/** A value's way from an output (left end) to an input (right end). */
function curve(a: Pt, b: Pt): string {
  const dx = bend(a, b);
  return `M ${a.x} ${a.y} C ${a.x + dx} ${a.y}, ${b.x - dx} ${b.y}, ${b.x} ${b.y}`;
}
