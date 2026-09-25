import { DisposableStore, listen } from '../../core/disposable';
import { h } from '../dom';

/** Rows built beyond each edge of the scroll window, so a quick scroll does not show blanks. */
const OVERSCAN = 30;

/**
 * A long list that builds only the rows in its scroll window (and OVERSCAN
 * either side), for lists that may run past a few hundred rows (CLAUDE.md
 * §6.2 rule 5): the command history, the warnings, a coordinate table of a
 * contour or ten thousand points. Rows are all one height, measured from the
 * first row built (so the type scale is followed); two spacers stand for the
 * rest. The rows' parent can be a list or a table body (`spacer` makes a
 * row that fits it); `scroller` is the element that scrolls.
 *
 * Rows are made by index, on demand: the caller keeps its data and nothing
 * is formatted for rows nobody sees.
 */
export class VirtualRows {
  private readonly d = new DisposableStore();
  private readonly parent: HTMLElement;
  private readonly scroller: HTMLElement;
  private readonly row: (i: number) => HTMLElement;
  private readonly before: HTMLElement;
  private readonly after: HTMLElement;
  private readonly setHeight: (el: HTMLElement, px: number) => void;
  private count = 0;
  private rowH = 0;
  private from = 0;
  private to = 0;
  private frame = 0;

  constructor(opts: {
    parent: HTMLElement;
    scroller: HTMLElement;
    row: (i: number) => HTMLElement;
    /** A row that takes the height of the rows it stands for (a `<li>`, or a `<tr>` with one cell across). */
    spacer: () => HTMLElement;
  }) {
    this.parent = opts.parent;
    this.scroller = opts.scroller;
    this.row = opts.row;
    this.before = opts.spacer();
    this.after = opts.spacer();
    for (const s of [this.before, this.after]) s.setAttribute('aria-hidden', 'true');
    // A table row's height is its cell's.
    this.setHeight = (el, px) => ((el.firstElementChild as HTMLElement | null) ?? el).style.setProperty('height', `${px}px`);
    this.d.add(listen(this.scroller, 'scroll', () => this.schedule()));
    const ro = new ResizeObserver(() => this.schedule());
    ro.observe(this.scroller);
    this.d.add(() => ro.disconnect());
    this.d.add(() => cancelAnimationFrame(this.frame));
  }

  /** New content: `count` rows. `end` scrolls to the last row (a log that follows its newest entry). */
  set(count: number, end = false): void {
    this.count = count;
    this.from = this.to = 0;
    this.parent.replaceChildren(this.before, this.after);
    this.render();
    if (end) {
      this.scroller.scrollTop = this.scroller.scrollHeight;
      this.render();
    }
  }

  /** Whether the list shows its last row (a log keeps following its end only then). */
  atEnd(): boolean {
    return this.scroller.scrollHeight - this.scroller.scrollTop - this.scroller.clientHeight < Math.max(4, this.rowH);
  }

  dispose(): void {
    this.d.dispose();
  }

  private schedule(): void {
    cancelAnimationFrame(this.frame);
    this.frame = requestAnimationFrame(() => this.render());
  }

  /** Builds the rows the scroll window needs and drops the ones it left behind. */
  private render(): void {
    const n = this.count;
    if (!n) {
      this.setHeight(this.before, 0);
      this.setHeight(this.after, 0);
      return;
    }
    if (!this.rowH) {
      // The first row sets the height of all of them.
      const probe = this.row(0);
      this.parent.insertBefore(probe, this.after);
      this.rowH = Math.max(1, probe.getBoundingClientRect().height);
      probe.remove();
    }
    // Rows start below whatever precedes them in the scroller (a table's head).
    const offset = this.before.offsetTop;
    const top = Math.max(0, this.scroller.scrollTop - offset);
    const view = this.scroller.clientHeight || 400;
    const from = Math.max(0, Math.floor(top / this.rowH) - OVERSCAN);
    const to = Math.min(n, Math.ceil((top + view) / this.rowH) + OVERSCAN);
    if (from === this.from && to === this.to && this.parent.childElementCount === 2 + to - from) return;
    const rows: HTMLElement[] = [];
    for (let i = from; i < to; i++) rows.push(this.row(i));
    this.parent.replaceChildren(this.before, ...rows, this.after);
    this.setHeight(this.before, from * this.rowH);
    this.setHeight(this.after, (n - to) * this.rowH);
    this.from = from;
    this.to = to;
  }
}

/** A spacer for a list (`<ol>`, `<ul>`). */
export const listSpacer = (): HTMLElement => h('li', { class: 'vrows__spacer' });

/** A spacer for a table body: one cell across `columns`. */
export const tableSpacer = (columns: number) => (): HTMLElement => h('tr', { class: 'vrows__spacer' }, h('td', { colspan: String(columns) }));
