import { DisposableStore, listen } from '../../core/disposable';
import { exprBuilderCatalog, type ExprField, type ExprItem, type ExprSection, type Fields } from '../../model/expression/builder';
import { h } from '../dom';
import { icon } from '../icons';
import { FLOW_DRAG_TYPE } from './FlowView';
import { KIND_STYLE } from './highlight';

/**
 * The builder's tree (DESIGN.md §7.16): the core's groups and entries,
 * searched as one types (names and English names, Turkish letters aside).
 * A click shows an entry's help; a double click or Enter puts it into the
 * expression. ↑/↓ move, → opens a group, ← closes it (or goes up to it).
 */
export interface BuilderTreeOptions {
  readonly fields: () => readonly ExprField[];
  /** What the services know of the objects (fields, `@` values, other layers); absent: the fields. */
  readonly schema?: () => Fields;
  readonly onSelect: (item: ExprItem) => void;
  readonly onInsert: (item: ExprItem) => void;
  /** Groups before the core's (the flow's values to write). */
  readonly extra?: () => readonly ExprSection[];
}

type Row = { readonly group: ExprSection; readonly item?: ExprItem };

const rowKey = (r: Row) => (r.item ? r.item.key : `group:${r.group.group}`);

export class BuilderTree {
  readonly el: HTMLElement;
  readonly search: HTMLInputElement;
  private readonly rowsEl: HTMLElement;
  private readonly opts: BuilderTreeOptions;
  private readonly d = new DisposableStore();
  private sections: ExprSection[] = [];
  private rows: Row[] = [];
  /** Open groups while nothing is searched (a search opens every group it keeps). */
  private readonly open = new Set<string>(['fields']);
  private current: string | null = null;

  constructor(opts: BuilderTreeOptions) {
    this.opts = opts;
    this.search = h('input', { class: 'field field--search xtree__search', type: 'search', placeholder: 'Ara: alan, işlev, değişken…', 'aria-label': 'İfade öğesi ara', spellcheck: 'false', autocomplete: 'off' });
    this.rowsEl = h('div', { class: 'xtree__rows', role: 'tree', tabindex: '0', 'aria-label': 'İfade öğeleri' });
    this.el = h('div', { class: 'xtree' }, this.search, this.rowsEl);
    this.d.add(listen(this.search, 'input', () => this.refresh()));
    this.d.add(listen<KeyboardEvent>(this.search, 'keydown', (e) => this.searchKey(e)));
    this.d.add(listen<KeyboardEvent>(this.rowsEl, 'keydown', (e) => this.key(e)));
    // Coming to the tree with the keyboard chooses its first entry.
    this.d.add(
      listen(this.rowsEl, 'focus', () => {
        const first = this.rows.find((r) => r.item)?.item;
        if (this.current === null && first) this.select(first.key, true);
      }),
    );
    this.refresh();
  }

  dispose(): void {
    this.d.dispose();
  }

  private get searching(): boolean {
    return this.search.value.trim() !== '';
  }

  /** Reads the groups again (the search, or the builder's view changed). */
  refresh(): void {
    const q = this.search.value.trim().toLocaleLowerCase('tr');
    const extra = (this.opts.extra?.() ?? [])
      .map((s) => ({ ...s, items: s.items.filter((i) => !q || i.label.toLocaleLowerCase('tr').includes(q)) }))
      .filter((s) => s.items.length);
    for (const s of extra) this.open.add(s.group);
    this.sections = [...extra, ...exprBuilderCatalog((this.opts.schema ?? this.opts.fields)(), this.search.value.trim())];
    this.render();
  }

  private render(): void {
    const rows: Row[] = [];
    for (const s of this.sections) {
      rows.push({ group: s });
      if (this.searching || this.open.has(s.group)) for (const item of s.items) rows.push({ group: s, item });
    }
    this.rows = rows;
    // Nothing is chosen until the tree is used (the help shows what is under the editor's cursor till then).
    if (!rows.some((r) => rowKey(r) === this.current)) this.current = null;
    if (!rows.length) {
      this.rowsEl.replaceChildren(h('div', { class: 'xtree__empty' }, `“${this.search.value.trim()}” için öğe yok.`));
      return;
    }
    this.rowsEl.replaceChildren(...rows.map((r, i) => this.row(r, i)));
    const at = rows.findIndex((r) => rowKey(r) === this.current);
    if (at >= 0) this.rowsEl.setAttribute('aria-activedescendant', `xtree-row-${at}`);
    else this.rowsEl.removeAttribute('aria-activedescendant');
  }

  private row(r: Row, i: number): HTMLElement {
    const key = rowKey(r);
    const selected = String(key === this.current);
    if (!r.item) {
      const expanded = this.searching || this.open.has(r.group.group);
      const el = h(
        'div',
        { class: 'xtree__row xtree__group', role: 'treeitem', id: `xtree-row-${i}`, 'aria-expanded': String(expanded), 'aria-selected': selected },
        h('span', { class: 'xtree__chev', 'data-open': expanded ? '' : null }, icon('chevronRight', 14)),
        h('span', { class: 'xtree__title' }, r.group.title),
        h('span', { class: 'xtree__count' }, String(r.group.items.length)),
      );
      el.addEventListener('click', () => {
        this.current = key;
        this.toggle(r.group.group);
      });
      return el;
    }
    const item = r.item;
    const style = item.key === 'lit:number' ? 'x-literal' : item.key === 'lit:text' ? 'x-text' : KIND_STYLE[item.kind];
    const el = h(
      'div',
      { class: 'xtree__row xtree__item', role: 'treeitem', id: `xtree-row-${i}`, 'aria-selected': selected, title: item.detail, draggable: 'true' },
      h('span', { class: `xtree__label ${style}` }, item.label),
      item.kind === 'field' ? h('span', { class: 'xtree__detail' }, item.detail.split(' · ')[0]) : null,
    );
    el.addEventListener('click', () => this.select(key, true));
    el.addEventListener('dblclick', () => this.opts.onInsert(item));
    // Dragged: onto the flow as a node, into the text as it is written.
    el.addEventListener('dragstart', (e) => {
      e.dataTransfer?.setData(FLOW_DRAG_TYPE, item.key);
      e.dataTransfer?.setData('text/plain', item.insert);
      if (e.dataTransfer) e.dataTransfer.effectAllowed = 'copy';
    });
    return el;
  }

  private toggle(group: string): void {
    if (this.searching) return;
    if (this.open.has(group)) this.open.delete(group);
    else this.open.add(group);
    this.render();
  }

  private select(key: string, show: boolean): void {
    this.current = key;
    this.rowsEl.querySelectorAll('.xtree__row').forEach((el, i) => {
      const on = this.rows[i] && rowKey(this.rows[i]) === key;
      el.setAttribute('aria-selected', String(on));
      if (on) {
        el.scrollIntoView({ block: 'nearest' });
        this.rowsEl.setAttribute('aria-activedescendant', el.id);
      }
    });
    const item = this.rows.find((r) => r.item?.key === key)?.item;
    if (show && item) this.opts.onSelect(item);
  }

  private searchKey(e: KeyboardEvent): void {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      this.rowsEl.focus();
      const first = this.rows.find((r) => r.item);
      if (first?.item) this.select(first.item.key, true);
    } else if (e.key === 'Enter') {
      // Enter in the search puts the first entry it found into the expression.
      const first = this.rows.find((r) => r.item)?.item;
      if (first) {
        e.preventDefault();
        this.opts.onInsert(first);
      }
    }
  }

  private key(e: KeyboardEvent): void {
    const at = this.rows.findIndex((r) => rowKey(r) === this.current);
    const row = this.rows[at];
    const go = (i: number) => {
      const r = this.rows[Math.max(0, Math.min(this.rows.length - 1, i))];
      if (r) this.select(rowKey(r), true);
    };
    switch (e.key) {
      case 'ArrowDown':
        go(at + 1);
        break;
      case 'ArrowUp':
        if (at <= 0) this.search.focus();
        else go(at - 1);
        break;
      case 'Home':
        go(0);
        break;
      case 'End':
        go(this.rows.length - 1);
        break;
      case 'ArrowRight':
        if (row && !row.item && !this.searching && !this.open.has(row.group.group)) this.toggle(row.group.group);
        else go(at + 1);
        break;
      case 'ArrowLeft':
        if (row?.item) this.select(`group:${row.group.group}`, false);
        else if (row && this.open.has(row.group.group)) this.toggle(row.group.group);
        break;
      case 'Enter':
      case ' ':
        if (row?.item) this.opts.onInsert(row.item);
        else if (row) this.toggle(row.group.group);
        break;
      default:
        return;
    }
    e.preventDefault();
    e.stopPropagation();
  }
}
