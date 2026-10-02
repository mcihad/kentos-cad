import { Emitter } from '../core/emitter';
import { Signal } from '../core/signal';
import type { LayerRenderer } from './style';

export type LineType = 'continuous' | 'dashed' | 'dashdot' | 'dotted';

export type PointSymbol = 'ring' | 'cross' | 'triangle';

/** How entity labels on a layer are drawn. Screen sizes are CSS px. */
export interface LabelStyle {
  /** centre of a polygon, top-left of its bounds, beside a point, or along a line. */
  placement: 'center' | 'corner' | 'beside' | 'along';
  size: number;
  /** Extra px per (px/m) of zoom, capped at maxSize. */
  grow?: number;
  maxSize?: number;
  weight?: 400 | 500 | 600;
  /** "{label}" is replaced with the entity label. */
  template?: string;
  /** Hide when the feature's smaller side is below this on screen. */
  minFeaturePx?: number;
  /** Visible zoom range in px per metre. */
  minScale?: number;
  maxScale?: number;
  ink?: 'fg' | 'fg-dim' | 'label';
}

export interface LayerStyle {
  /** Hex colour, or a theme token: "fg" (main ink) / "fg-dim". */
  color: string;
  lineType: LineType;
  /** Plot line weight in mm. */
  lineWeight: number;
  /** Optional polygon fill (hex with alpha, e.g. "#7FB2E526"). */
  fill?: string;
  /** Symbol and diameter (px) for point entities. */
  point?: { symbol: PointSymbol; size: number };
  label?: LabelStyle;
  /**
   * Whether clicking inside a closed shape selects it (GIS-style area pick).
   * Off for frames and reference outlines that enclose everything.
   */
  pickInterior?: boolean;
  /**
   * The layer's style (docs/STYLE.md): which symbols its objects get. When
   * absent, the simple look above (colour, line type, weight, fill) is used.
   */
  renderer?: LayerRenderer;
}

export interface LayerNode {
  id: string;
  name: string;
  type: 'group' | 'layer';
  visible: boolean;
  locked: boolean;
  expanded: boolean;
  style: LayerStyle;
  children: LayerNode[];
  /** A layer's own snapping (docs/adr/0163 §4): off, or only these kinds; absent, the general kinds. Never a group's. */
  snap?: LayerSnap;
}

/** A layer's own snapping: `{ off: true }` or `{ kinds: [...] }` (contracts' `LayerSnap`, exactly one of the two). */
export type LayerSnap = { off: true; kinds?: never } | { kinds: string[]; off?: never };

/** Whether two layer snaps are the same: both none, both off, or the same kinds in the same order. */
export function sameSnap(a: LayerSnap | undefined | null, b: LayerSnap | undefined | null): boolean {
  if (!a || !b) return !a && !b;
  return !!a.off === !!b.off && (a.kinds ?? []).join() === (b.kinds ?? []).join();
}

export type LayerInit = Partial<Omit<LayerNode, 'children' | 'style'>> & {
  name: string;
  style?: Partial<LayerStyle>;
  children?: LayerInit[];
};

export const LINE_TYPE_LABEL: Record<LineType, string> = {
  continuous: 'Sürekli',
  dashed: 'Kesikli',
  dashdot: 'Noktalı kesik',
  dotted: 'Noktalı',
};

interface LayerEvents {
  /** Structure changed (add/remove/rename/reorder). */
  structure: void;
  /** Visibility / lock / style changed — carries affected leaf layer ids. */
  state: { ids: string[] };
  /** A group was opened or closed in the tree: a view change, not an edit. */
  expanded: { id: string };
}

let uid = 0;
const defaultStyle: LayerStyle = { color: 'fg', lineType: 'continuous', lineWeight: 0.18 };

export class LayerStore {
  readonly events = new Emitter<LayerEvents>();
  /** Bumped on any change — cheap subscription for UI lists. */
  readonly version = new Signal(0);
  readonly active: Signal<string>;
  private roots: LayerNode[] = [];
  private index = new Map<string, LayerNode>();
  private parents = new Map<string, LayerNode | null>();

  constructor(init: LayerInit[], activeId: string) {
    this.roots = init.map((n) => this.build(n, null));
    this.active = new Signal(activeId);
  }

  private build(n: LayerInit, parent: LayerNode | null): LayerNode {
    const node = this.make(n);
    this.register(node, parent);
    return node;
  }

  /**
   * A node as `add` makes it, not yet in the tree: its defaults, and an id
   * from the counter when none is given (with its children, made the same
   * way). The document's undoable `addLayer` records it.
   */
  make(n: LayerInit): LayerNode {
    // Ids read from a file ("layer-12") keep the counter ahead of them, so new layers never collide.
    const m = n.id ? /^layer-(\d+)$/.exec(n.id) : null;
    if (m) uid = Math.max(uid, Number(m[1]));
    const node: LayerNode = {
      id: n.id ?? `layer-${++uid}`,
      name: n.name,
      type: n.type ?? (n.children ? 'group' : 'layer'),
      visible: n.visible ?? true,
      locked: n.locked ?? false,
      expanded: n.expanded ?? true,
      style: { ...defaultStyle, ...n.style },
      children: [],
      // A layer's own snapping (docs/adr/0163 §4); a group keeps none.
      ...(n.snap && (n.type ?? (n.children ? 'group' : 'layer')) === 'layer' && { snap: n.snap.kinds ? { kinds: [...n.snap.kinds] } : { off: true } }),
    };
    node.children = (n.children ?? []).map((c) => this.make(c));
    return node;
  }

  /** Puts `node` and everything under it into the index, under `parent`; the leaves' ids go to `leaves`. */
  private register(node: LayerNode, parent: LayerNode | null, leaves: string[] = []): string[] {
    this.index.set(node.id, node);
    this.parents.set(node.id, parent);
    if (node.type === 'layer') leaves.push(node.id);
    for (const c of node.children) this.register(c, node, leaves);
    return leaves;
  }

  get tree(): readonly LayerNode[] {
    return this.roots;
  }

  /**
   * Replaces the whole tree (a drawing opened from a file). The active
   * layer is kept when it exists as a layer, else the first layer.
   */
  reset(init: readonly LayerInit[], activeId: string): void {
    this.index.clear();
    this.parents.clear();
    this.roots = init.map((n) => this.build(n, null));
    const active = this.get(activeId)?.type === 'layer' ? activeId : (this.leaves()[0]?.id ?? activeId);
    this.active.set(active);
    this.events.emit('structure', undefined);
    this.events.emit('state', { ids: this.leaves().map((l) => l.id) });
    this.version.update((v) => v + 1);
  }

  get(id: string): LayerNode | undefined {
    return this.index.get(id);
  }

  parentOf(id: string): LayerNode | null {
    return this.parents.get(id) ?? null;
  }

  /** Leaf layers in draw order (tree order, top of list drawn last = on top). */
  leaves(): LayerNode[] {
    const out: LayerNode[] = [];
    const walk = (nodes: LayerNode[]) => {
      for (const n of nodes) n.type === 'layer' ? out.push(n) : walk(n.children);
    };
    walk(this.roots);
    return out;
  }

  leavesOf(id: string): LayerNode[] {
    const n = this.get(id);
    if (!n) return [];
    if (n.type === 'layer') return [n];
    const out: LayerNode[] = [];
    const walk = (x: LayerNode) => (x.type === 'layer' ? out.push(x) : x.children.forEach(walk));
    walk(n);
    return out;
  }

  isVisible(id: string): boolean {
    for (let n: LayerNode | null | undefined = this.get(id); n; n = this.parentOf(n.id)) if (!n.visible) return false;
    return true;
  }

  isLocked(id: string): boolean {
    for (let n: LayerNode | null | undefined = this.get(id); n; n = this.parentOf(n.id)) if (n.locked) return true;
    return false;
  }

  path(id: string): string {
    const names: string[] = [];
    for (let n: LayerNode | null | undefined = this.get(id); n; n = this.parentOf(n.id)) names.unshift(n.name);
    return names.join(' / ');
  }

  setVisible(id: string, visible: boolean): void {
    const n = this.get(id);
    if (!n || n.visible === visible) return;
    n.visible = visible;
    this.changedState(id);
  }

  toggleVisible(id: string): void {
    const n = this.get(id);
    if (n) this.setVisible(id, !n.visible);
  }

  toggleLocked(id: string): void {
    const n = this.get(id);
    if (!n) return;
    n.locked = !n.locked;
    this.changedState(id);
  }

  /**
   * A layer's own snapping (docs/adr/0163 §4): `snap` on the layer, or on every layer of the group (a group keeps
   * none); null takes it off, back to the general kinds. An edit when anything changed, not an undo step (as a lock).
   */
  setSnap(id: string, snap: LayerSnap | null): void {
    let changed = false;
    for (const l of this.leavesOf(id)) {
      if (sameSnap(l.snap, snap)) continue;
      if (snap) l.snap = snap.kinds ? { kinds: [...snap.kinds] } : { off: true };
      else delete l.snap;
      changed = true;
    }
    if (changed) this.changedState(id);
  }

  /** Show only this node (and its ancestors); hide every other leaf. */
  isolate(id: string): void {
    // An unknown id hides nothing (docs/adr/0020).
    if (!this.get(id)) return;
    const keep = new Set<string>();
    for (let n: LayerNode | null | undefined = this.get(id); n; n = this.parentOf(n.id)) keep.add(n.id);
    for (const l of this.leavesOf(id)) keep.add(l.id);
    for (const n of this.index.values()) n.visible = keep.has(n.id) || this.isDescendant(n.id, id);
    this.events.emit('state', { ids: this.leaves().map((l) => l.id) });
    this.version.update((v) => v + 1);
  }

  showAll(): void {
    // Everything already shown: nothing changes, not an edit (docs/adr/0020).
    if ([...this.index.values()].every((n) => n.visible)) return;
    for (const n of this.index.values()) n.visible = true;
    this.events.emit('state', { ids: this.leaves().map((l) => l.id) });
    this.version.update((v) => v + 1);
  }

  setExpanded(id: string, expanded: boolean): void {
    const n = this.get(id);
    if (!n || n.expanded === expanded) return;
    n.expanded = expanded;
    this.events.emit('expanded', { id });
    this.version.update((v) => v + 1);
  }

  setStyle(id: string, style: Partial<LayerStyle>): void {
    const n = this.get(id);
    if (!n) return;
    Object.assign(n.style, style);
    this.changedState(id);
  }

  /** Sets a layer's whole style (undo and redo restore it exactly). */
  replaceStyle(id: string, style: LayerStyle): void {
    const n = this.get(id);
    if (!n) return;
    n.style = structuredClone(style);
    this.changedState(id);
  }

  rename(id: string, name: string): void {
    const n = this.get(id);
    // The same name again changes nothing, not an edit (docs/adr/0020).
    if (!n || !name.trim() || name.trim() === n.name) return;
    n.name = name.trim();
    this.events.emit('structure', undefined);
    this.version.update((v) => v + 1);
  }

  setActive(id: string): void {
    const n = this.get(id);
    if (n?.type === 'layer') this.active.set(id);
  }

  add(init: LayerInit, parentId?: string | null): LayerNode {
    const container = this.get(this.containerFor(parentId) ?? '') ?? null;
    const node = this.build(init, container);
    (container ? container.children : this.roots).push(node);
    if (container) container.expanded = true;
    this.events.emit('structure', undefined);
    this.version.update((v) => v + 1);
    return node;
  }

  /**
   * The group a node added beside or into `parentId` goes into: that group,
   * the layer's group, or null for the top (a top layer or an unknown id).
   */
  containerFor(parentId?: string | null): string | null {
    const parent = parentId ? this.get(parentId) : null;
    const container = parent && parent.type === 'group' ? parent : parent ? this.parentOf(parent.id) : null;
    return container?.id ?? null;
  }

  /** Where a node sits: its parent group (null at the top) and its place among the parent's nodes. Null for an unknown id. */
  placeOf(id: string): { parent: string | null; index: number } | null {
    const node = this.get(id);
    if (!node) return null;
    const parent = this.parentOf(id);
    return { parent: parent?.id ?? null, index: (parent ? parent.children : this.roots).indexOf(node) };
  }

  /**
   * Takes a node out of the tree with everything under it (the document's
   * undoable layer ops, `CadDocument.removeLayer` and the undo of
   * `addLayer`; nothing else calls it). When the active layer goes with it,
   * the first layer of the tree becomes active, as `reset` does. An unknown
   * id changes nothing.
   */
  detach(id: string): void {
    const node = this.get(id);
    if (!node) return;
    const parent = this.parentOf(id);
    const list = parent ? parent.children : this.roots;
    list.splice(list.indexOf(node), 1);
    const leaves: string[] = [];
    const forget = (n: LayerNode): void => {
      this.index.delete(n.id);
      this.parents.delete(n.id);
      if (n.type === 'layer') leaves.push(n.id);
      n.children.forEach(forget);
    };
    forget(node);
    if (this.get(this.active.value)?.type !== 'layer') this.active.set(this.leaves()[0]?.id ?? this.active.value);
    this.events.emit('structure', undefined);
    this.events.emit('state', { ids: leaves });
    this.version.update((v) => v + 1);
  }

  /**
   * Puts a node where `detach` took it from (undo), or where the document's
   * `addLayer` puts a new one: a copy of `node`, so the one a history step
   * keeps stays as it was, with its children, flags and style. A parent that
   * is gone puts it at the top. A node whose id the tree already has is not
   * put in twice (nothing changes).
   */
  attach(node: LayerNode, parent: string | null, index: number): void {
    if (this.get(node.id)) return;
    const container = parent ? (this.get(parent) ?? null) : null;
    const copy = structuredClone(node);
    const leaves = this.register(copy, container);
    const list = container ? container.children : this.roots;
    list.splice(Math.min(index, list.length), 0, copy);
    this.events.emit('structure', undefined);
    this.events.emit('state', { ids: leaves });
    this.version.update((v) => v + 1);
  }

  uniqueName(base: string): string {
    const names = new Set([...this.index.values()].map((n) => n.name));
    if (!names.has(base)) return base;
    for (let i = 2; ; i++) if (!names.has(`${base} ${i}`)) return `${base} ${i}`;
  }

  private isDescendant(id: string, ancestorId: string): boolean {
    for (let n = this.parentOf(id); n; n = this.parentOf(n.id)) if (n.id === ancestorId) return true;
    return false;
  }

  private changedState(id: string): void {
    this.events.emit('state', { ids: this.leavesOf(id).map((l) => l.id) });
    this.version.update((v) => v + 1);
  }
}
