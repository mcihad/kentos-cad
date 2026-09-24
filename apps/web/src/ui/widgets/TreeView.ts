import type { Disposable } from '../../core/disposable';
import { h } from '../dom';
import { icon } from '../icons';
import { hideTooltip, tooltip, type TooltipContent } from './tooltip';

export interface TreeAdapter<T> {
  id(node: T): string;
  children(node: T): readonly T[];
  isExpanded(node: T): boolean;
  setExpanded(node: T, expanded: boolean): void;
  /**
   * Fill the row's content cell. Rows are built only while they are in the
   * scroll window: what is returned is released when the row goes (a
   * tooltip on it, say).
   */
  renderRow(node: T, row: HTMLElement): Disposable | void;
  /** A row left the page (scrolled away or rendered again): forget what refers to it. */
  releaseRow?(node: T, row: HTMLElement): void;
  /** Click or keyboard focus moved to a row. */
  onSelect?(node: T): void;
  /** Double click / Enter. */
  onActivate?(node: T): void;
  /** Space. */
  onToggle?(node: T): void;
  onRename?(node: T): void;
  onContextMenu?(node: T, e: MouseEvent): void;
  /** Keep a node while filtering (ancestors of matches are kept automatically). */
  matches?(node: T, query: string): boolean;
  /** Text when nothing is listed (default speaks of layers). */
  empty?(filtered: boolean): string;
  /**
   * Browsing trees (libraries): a click on a node with children opens it,
   * a second click on the selected node closes it; the caret still toggles.
   */
  readonly clickToggles?: boolean;
  /** Hover tooltip for a row (e.g. the full name when it is cut short); null shows none. */
  tip?(node: T, row: HTMLElement): TooltipContent | null;
}

/** A listed node: one line of the tree, whether or not its row is built. */
interface Line<T> {
  readonly node: T;
  readonly id: string;
  readonly depth: number;
  readonly hasKids: boolean;
  readonly expanded: boolean;
  /** Its place among its listed siblings (1-based) and their number: a screen reader's “3 of 12”. */
  readonly pos: number;
  readonly size: number;
}

/** A row on the page. */
interface Built<T> {
  readonly node: T;
  readonly row: HTMLElement;
  readonly content: HTMLElement;
  readonly release: Disposable[];
}

/** Rows built beyond each edge of the scroll window: a short scroll needs none. */
const OVERSCAN = 40;
/** Row height until the first measurement (`--row-h` at scale 1). */
const GUESS_ROW_H = 28;

/**
 * Generic keyboard-accessible tree (role=tree), rendered from the model.
 * The listed nodes are kept as data (`lines`); only the rows in the scroll
 * window, and OVERSCAN more either side, are built, between two spacers
 * that stand for the rest. A tree of 300 layers costs what the screen
 * shows, and scrolling builds the rows that come in. Every row is one
 * `--row-h` high, read from a hidden probe row (again when the UI scale
 * changes it). `focus` and `rowOf` scroll a line in, so its row exists.
 */
export class TreeView<T> {
  readonly el: HTMLElement;
  private readonly adapter: TreeAdapter<T>;
  private lines: Line<T>[] = [];
  /** Line index by node id. */
  private at = new Map<string, number>();
  /** The rows on the page by node id: the window's. */
  private readonly built = new Map<string, Built<T>>();
  private readonly above: HTMLElement;
  private readonly below: HTMLElement;
  private readonly probe: HTMLElement;
  private emptyEl: HTMLElement | null = null;
  private rowH = 0;
  private focusedId: string | null = null;
  private query = '';
  private roots: readonly T[] = [];
  private frame = 0;
  private readonly sizes: ResizeObserver;

  constructor(adapter: TreeAdapter<T>, label: string) {
    this.adapter = adapter;
    this.above = h('div', { class: 'tree__spacer', 'aria-hidden': 'true' });
    this.below = h('div', { class: 'tree__spacer', 'aria-hidden': 'true' });
    this.probe = h('div', { class: 'tree__row tree__probe', 'aria-hidden': 'true' });
    this.el = h('div', { class: 'tree', role: 'tree', 'aria-label': label, tabindex: '0' }, this.probe, this.above, this.below);
    this.el.addEventListener('keydown', (e) => this.onKey(e));
    this.el.addEventListener('focus', () => {
      if (!this.focusedId && this.lines.length) this.focus(this.lines[0].id);
    });
    this.el.addEventListener('scroll', () => this.schedule(), { passive: true });
    // The window grows or shrinks with the panel; the rows with the UI scale (the probe).
    this.sizes = new ResizeObserver(() => {
      const measured = this.probe.getBoundingClientRect().height;
      if (measured > 0) this.rowH = measured;
      this.paint();
    });
    this.sizes.observe(this.el);
    this.sizes.observe(this.probe);
  }

  setFilter(q: string): void {
    this.query = q.trim().toLocaleLowerCase('tr-TR');
    this.render(this.roots);
  }

  render(roots: readonly T[]): void {
    this.roots = roots;
    const a = this.adapter;
    // Rows are replaced: a tooltip of an old row would stay open with nothing under the pointer.
    hideTooltip(this.el);
    for (const id of [...this.built.keys()]) this.release(id);
    this.emptyEl?.remove();
    this.emptyEl = null;
    const q = this.query;
    const keep = (n: T): boolean => !q || !!a.matches?.(n, q) || a.children(n).some(keep);
    const lines: Line<T>[] = [];
    const walk = (nodes: readonly T[], depth: number) => {
      const listed = nodes.filter(keep);
      listed.forEach((n, i) => {
        const kids = a.children(n);
        const hasKids = kids.length > 0;
        const expanded = hasKids && (a.isExpanded(n) || !!q);
        lines.push({ node: n, id: a.id(n), depth, hasKids, expanded, pos: i + 1, size: listed.length });
        if (expanded) walk(kids, depth + 1);
      });
    };
    walk(roots, 0);
    this.lines = lines;
    this.at = new Map(lines.map((l, i) => [l.id, i]));
    if (!lines.length) {
      this.emptyEl = h('div', { class: 'tree__empty' }, a.empty?.(!!q) ?? (q ? 'Aramayla eşleşen katman yok.' : 'Katman yok.'));
      this.el.append(this.emptyEl);
    }
    this.paint();
  }

  focus(id: string, moveDom = true): void {
    const changed = this.focusedId !== id;
    this.mark(id);
    const i = this.at.get(id);
    if (i === undefined) return;
    if (moveDom) this.reveal(i);
    if (changed) this.adapter.onSelect?.(this.lines[i].node);
  }

  /** Marks a row selected without telling the adapter (the caller already knows). */
  mark(id: string | null): void {
    const before = this.focusedId;
    this.focusedId = id;
    if (before !== null) this.built.get(before)?.row.setAttribute('aria-selected', 'false');
    if (id !== null) this.built.get(id)?.row.setAttribute('aria-selected', 'true');
  }

  /** The row of a listed node, scrolled into the window (so it is built); undefined when not listed. */
  rowOf(id: string): HTMLElement | undefined {
    const i = this.at.get(id);
    if (i === undefined) return undefined;
    this.reveal(i);
    return this.built.get(id)?.row;
  }

  /** Stops watching sizes and releases the rows (the tree's owner goes away). */
  dispose(): void {
    this.sizes.disconnect();
    cancelAnimationFrame(this.frame);
    for (const id of [...this.built.keys()]) this.release(id);
  }

  private schedule(): void {
    if (this.frame) return;
    this.frame = requestAnimationFrame(() => {
      this.frame = 0;
      this.paint();
    });
  }

  /** Scrolls the tree just enough to show line i, and builds its row. */
  private reveal(i: number): void {
    const rowH = this.rowHeight();
    const top = i * rowH;
    const view = this.el.clientHeight;
    if (top < this.el.scrollTop) this.el.scrollTop = top;
    else if (view && top + rowH > this.el.scrollTop + view) this.el.scrollTop = top + rowH - view;
    this.paint(i);
  }

  private rowHeight(): number {
    if (!this.rowH) this.rowH = this.probe.getBoundingClientRect().height;
    return this.rowH || GUESS_ROW_H;
  }

  /**
   * Builds the rows of the scroll window (and OVERSCAN more either side,
   * and line `keep`), releases the others and sizes the spacers. Reads the
   * scroll position first, then only writes.
   */
  private paint(keep = -1): void {
    const n = this.lines.length;
    const rowH = this.rowHeight();
    const view = this.el.clientHeight;
    const top = Math.min(this.el.scrollTop, Math.max(0, n * rowH - view));
    let from = Math.max(0, Math.floor(top / rowH) - OVERSCAN);
    let to = Math.min(n, Math.ceil((top + view) / rowH) + OVERSCAN);
    if (keep >= 0 && keep < n) {
      from = Math.min(from, keep);
      to = Math.max(to, keep + 1);
    }
    for (const [id] of this.built) {
      const i = this.at.get(id);
      if (i === undefined || i < from || i >= to) this.release(id);
    }
    // The window's rows in order between the spacers; rows already there stay put.
    let next: ChildNode | null = this.above.nextSibling;
    for (let i = from; i < to; i++) {
      const line = this.lines[i];
      let b = this.built.get(line.id);
      if (!b) {
        b = this.build(line);
        this.built.set(line.id, b);
      }
      if (b.row === next) next = next.nextSibling;
      else this.el.insertBefore(b.row, next);
    }
    this.above.style.height = `${from * rowH}px`;
    this.below.style.height = `${Math.max(0, n - to) * rowH}px`;
  }

  private build(line: Line<T>): Built<T> {
    const a = this.adapter;
    const { node: n, id, depth, hasKids, expanded } = line;
    const caret = h('span', { class: 'tree__caret', 'data-empty': hasKids ? null : '' }, hasKids ? icon(expanded ? 'chevronDown' : 'chevronRight', 14) : null);
    const content = h('div', { class: 'tree__content' });
    const row = h(
      'div',
      {
        class: 'tree__row',
        role: 'treeitem',
        'aria-level': String(depth + 1),
        'aria-setsize': String(line.size),
        'aria-posinset': String(line.pos),
        'aria-expanded': hasKids ? String(expanded) : null,
        'aria-selected': String(id === this.focusedId),
        style: `--depth:${depth}`,
        dataset: { id },
      },
      caret,
      content,
    );
    const release: Disposable[] = [];
    const own = a.renderRow(n, content);
    if (own) release.push(own);
    if (a.tip) release.push(tooltip(row, () => a.tip!(n, row), 'right'));
    caret.addEventListener('click', (e) => {
      e.stopPropagation();
      if (hasKids) a.setExpanded(n, !a.isExpanded(n));
    });
    row.addEventListener('click', (e) => {
      if ((e.target as HTMLElement).closest('input')) return;
      const again = this.focusedId === id;
      this.focus(id, false);
      if (a.clickToggles && hasKids && (again || !a.isExpanded(n))) a.setExpanded(n, !a.isExpanded(n));
    });
    row.addEventListener('dblclick', (e) => {
      if ((e.target as HTMLElement).closest('button, input')) return;
      if (a.clickToggles) return;
      if (hasKids && !a.onActivate) a.setExpanded(n, !a.isExpanded(n));
      else a.onActivate?.(n);
    });
    row.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      this.focus(id, false);
      a.onContextMenu?.(n, e);
    });
    return { node: n, row, content, release };
  }

  private release(id: string): void {
    const b = this.built.get(id);
    if (!b) return;
    this.built.delete(id);
    hideTooltip(b.row);
    for (const d of b.release) d();
    b.row.remove();
    this.adapter.releaseRow?.(b.node, b.content);
  }

  private onKey(e: KeyboardEvent): void {
    if ((e.target as HTMLElement).closest('input')) return;
    const a = this.adapter;
    const i = this.focusedId === null ? -1 : (this.at.get(this.focusedId) ?? -1);
    const cur = this.lines[i]?.node;
    const go = (k: number) => {
      const line = this.lines[Math.max(0, Math.min(this.lines.length - 1, k))];
      if (line) this.focus(line.id);
    };
    const handled = () => {
      e.preventDefault();
      e.stopPropagation();
    };
    switch (e.key) {
      case 'ArrowDown':
        handled();
        return go(i + 1);
      case 'ArrowUp':
        handled();
        return go(i - 1);
      case 'Home':
        handled();
        return go(0);
      case 'End':
        handled();
        return go(this.lines.length - 1);
      case 'ArrowRight':
        handled();
        if (cur && a.children(cur).length) a.isExpanded(cur) ? go(i + 1) : a.setExpanded(cur, true);
        return;
      case 'ArrowLeft': {
        handled();
        if (!cur) return;
        if (a.children(cur).length && a.isExpanded(cur)) return a.setExpanded(cur, false);
        // To the parent: the nearest line above that sits one level up.
        const depth = this.lines[i].depth;
        for (let k = i - 1; k >= 0; k--) if (this.lines[k].depth < depth) return go(k);
        return;
      }
      case 'Enter':
        handled();
        if (cur) a.onActivate?.(cur);
        return;
      case ' ':
        handled();
        if (cur) a.onToggle?.(cur);
        return;
      case 'F2':
        handled();
        if (cur) a.onRename?.(cur);
        return;
    }
  }
}
