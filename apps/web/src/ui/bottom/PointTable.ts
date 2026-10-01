import type { AppContext } from '../../app/context';
import { listen } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import type { Entity, PointEntity } from '../../model/entities';
import { pointTable, type SortColumn, type TableQuery, type TableRow } from '../../model/ops/pointEditor';
import { Component } from '../Component';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { Dropdown } from '../widgets/Dropdown';
import type { MenuItem } from '../widgets/PopupMenu';
import { tooltip } from '../widgets/tooltip';
import { tableSpacer, VirtualRows } from '../widgets/VirtualRows';

/**
 * Noktalar, the bottom panel's point editor (docs/adr/0153 §1–§2): every point of the drawing in a table, its search,
 * layer and selection filters and its column sort the shared core's (`pointTable`, the same order the desktop shows).
 * A click selects the point in the drawing (Ctrl turns one over, Shift takes the run from the last click); the
 * drawing's selection shows in the rows, the first selected row scrolled into view when it changes (once: scrolling
 * away stays). Göster, or a double click on a row's number, zooms to the selected points. The desktop's is
 * `apps/desktop/src/points/`.
 */

/** The columns: their header, the core's sort key (none: Sıra, the drawing's order) and whether they hold numbers. */
export const POINT_COLUMNS: readonly { label: string; sort: SortColumn | null; numeric: boolean }[] = [
  { label: 'Sıra', sort: null, numeric: true },
  { label: 'Ad', sort: 'name', numeric: false },
  { label: 'Y (sağa)', sort: 'east', numeric: true },
  { label: 'X (yukarı)', sort: 'north', numeric: true },
  { label: 'Z (kot)', sort: 'z', numeric: true },
  { label: 'Kod', sort: 'code', numeric: false },
  { label: 'Katman', sort: 'layer', numeric: false },
];

export const POINT_TEXTS = {
  search: 'Ad ya da kod ara',
  searchHint: 'Adda ya da kodda arar; * herhangi bir dizi: P1*, *0',
  allLayers: 'Bütün katmanlar',
  onlySelected: 'Yalnız seçililer',
  show: 'Göster',
  showHint: 'Seçili noktalara yakınlaştırır',
  none: 'Çizimde nokta yok. Nokta aracıyla ya da Nokta listesi içe aktar ile ekleyin.',
  noMatch: 'Süzgece uyan nokta yok.',
} as const;

/** The table's query, kept for the session: leaving the tab and coming back keeps it. */
const kept: TableQuery = { search: '', layer: null, onlySelected: false, sort: null, descending: false };

/** The query as a new session starts it (the pictures start each scene from it). */
export function resetPointTable(): void {
  Object.assign(kept, { search: '', layer: null, onlySelected: false, sort: null, descending: false });
}

/** A header click (docs/adr/0153 §2): ascending, then descending, then the drawing's order; Sıra is the drawing's order. */
export function nextSort(q: Pick<TableQuery, 'sort' | 'descending'>, column: SortColumn | null): Pick<TableQuery, 'sort' | 'descending'> {
  if (column === null || (q.sort === column && q.descending)) return { sort: null, descending: false };
  return { sort: column, descending: q.sort === column };
}

/**
 * What a click on the row at `at` (in the order shown) selects: that point alone; with Ctrl the selection with it
 * turned over; with Shift the run from `anchor` (the last click without Shift) to it. Ids of `shown` in that order.
 */
export function clickPick(selected: ReadonlySet<number>, shown: readonly number[], at: number, anchor: number | null, how: { ctrl: boolean; shift: boolean }): number[] {
  const id = shown[at];
  if (how.shift && anchor !== null && anchor < shown.length) {
    const [a, b] = anchor <= at ? [anchor, at] : [at, anchor];
    return shown.slice(a, b + 1);
  }
  if (how.ctrl) return selected.has(id) ? [...selected].filter((s) => s !== id) : [...selected, id];
  return [id];
}

/** A point as the table reads it (the core's `TableRow`): its label, place, elevation, `Kod`, layer name, selection. */
export function rowOf(e: PointEntity, layerName: string, selected: boolean): TableRow {
  return { name: e.label ?? null, east: e.p.x, north: e.p.y, z: e.z ?? null, code: e.attrs.Kod ?? null, layer: layerName, selected };
}

export class PointTable extends Component {
  readonly el: HTMLElement;
  private readonly ctx: AppContext;
  private readonly scroller: HTMLElement;
  private readonly body = h('tbody');
  private readonly heads: HTMLElement[] = [];
  private readonly count = h('span', { class: 'ptable__count num' });
  private readonly empty = h('div', { class: 'empty empty--inline ptable__empty' });
  private readonly layerPick: Dropdown;
  private readonly onlyBox: HTMLInputElement;
  private readonly showBtn: HTMLButtonElement;
  private readonly rows: VirtualRows;
  /** The points in the drawing's order and, in the order shown, their indices. */
  private points: PointEntity[] = [];
  private shown: number[] = [];
  /** The ids shown, in that order (for clicks). */
  private ids: number[] = [];
  /** The last click without Shift, in the order shown. */
  private anchor: number | null = null;
  /** The first selected row last scrolled to (its id and place): a change of it scrolls, once. */
  private revealed: string | null = null;
  private pending = false;

  constructor(ctx: AppContext) {
    super();
    this.ctx = ctx;
    const search = h('input', { class: 'field field--search ptable__search', type: 'search', placeholder: POINT_TEXTS.search, 'aria-label': POINT_TEXTS.search, value: kept.search, spellcheck: 'false' });
    this.d.add(tooltip(search, () => ({ title: POINT_TEXTS.search, description: POINT_TEXTS.searchHint }), 'top'));
    this.d.add(
      listen(search, 'input', () => {
        kept.search = search.value;
        this.schedule();
      }),
    );
    this.layerPick = new Dropdown({ ariaLabel: 'Katman', width: 190, className: 'ptable__layer', items: () => this.layerItems() });
    this.onlyBox = h('input', { type: 'checkbox', checked: kept.onlySelected });
    this.d.add(
      listen(this.onlyBox, 'change', () => {
        kept.onlySelected = this.onlyBox.checked;
        this.schedule();
      }),
    );
    this.showBtn = h('button', { class: 'btn btn--small ptable__show', type: 'button' }, icon('zoomSelection', 14), h('span', null, POINT_TEXTS.show));
    this.d.add(listen(this.showBtn, 'click', () => ctx.commands.execute('view.zoomSelection')));
    this.d.add(tooltip(this.showBtn, () => ({ title: POINT_TEXTS.show, description: POINT_TEXTS.showHint }), 'top'));

    const head = h(
      'tr',
      null,
      POINT_COLUMNS.map((c) => {
        const th = h('th', { class: c.numeric ? 'num' : null, scope: 'col' }, h('button', { class: 'ptable__sort', type: 'button' }, h('span', null, c.label), h('span', { class: 'ptable__arrow' })));
        this.d.add(
          listen(th.firstElementChild as HTMLElement, 'click', () => {
            Object.assign(kept, nextSort(kept, c.sort));
            this.schedule();
          }),
        );
        this.heads.push(th);
        return th;
      }),
    );
    this.scroller = h('div', { class: 'ptable__scroll' }, h('table', null, h('thead', null, head), this.body), this.empty);
    this.el = h(
      'div',
      { class: 'ptable' },
      h(
        'div',
        { class: 'ptable__bar' },
        search,
        this.layerPick.el,
        h('label', { class: 'io-check ptable__only' }, this.onlyBox, POINT_TEXTS.onlySelected),
        h('span', { class: 'ptable__gap' }),
        this.count,
        this.showBtn,
      ),
      this.scroller,
    );
    this.rows = new VirtualRows({ parent: this.body, scroller: this.scroller, row: (i) => this.row(i), spacer: tableSpacer(POINT_COLUMNS.length) });
    this.d.add(() => this.rows.dispose());

    // A click selects; a double click on the number zooms to it.
    this.d.add(
      listen<MouseEvent>(this.body, 'click', (e) => {
        const at = this.rowAt(e);
        if (at === null) return;
        const how = { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey };
        const ids = clickPick(ctx.selection.ids.value, this.ids, at, this.anchor, how);
        if (!how.shift) this.anchor = at;
        ctx.selection.set(ids);
      }),
    );
    this.d.add(
      listen<MouseEvent>(this.body, 'dblclick', (e) => {
        const at = this.rowAt(e);
        if (at === null || !(e.target as HTMLElement).closest('td:first-child')) return;
        ctx.selection.set([this.ids[at]]);
        ctx.commands.execute('view.zoomSelection');
      }),
    );
    this.d.add(watchAll([ctx.selection.ids, ctx.format.changed], () => this.schedule()));
    this.d.add(ctx.doc.events.on('changed', () => this.schedule()));
    this.d.add(ctx.doc.layers.version.subscribe(() => this.schedule()));
    // Built once the table is in the page (the first row's height is measured there).
    queueMicrotask(() => this.refresh());
  }

  /** The row index (in the order shown) under a mouse event, or null. */
  private rowAt(e: MouseEvent): number | null {
    const tr = (e.target as HTMLElement).closest<HTMLElement>('tr[data-at]');
    return tr ? Number(tr.dataset.at) : null;
  }

  /** Changes come in bursts (an undo changes many objects): one refresh after them. */
  private schedule(): void {
    if (this.pending) return;
    this.pending = true;
    queueMicrotask(() => {
      this.pending = false;
      this.refresh();
    });
  }

  /** The layers that hold points, in the layer list's order, each with its count; Bütün katmanlar first. */
  private layerItems(): MenuItem[] {
    const counts = new Map<string, number>();
    for (const e of this.points) counts.set(e.layerId, (counts.get(e.layerId) ?? 0) + 1);
    const { layers } = this.ctx.doc;
    const named = [...counts.keys()].map((id) => ({ id, name: layers.get(id)?.name ?? id }));
    const pick = (name: string | null) => () => {
      kept.layer = name;
      this.schedule();
    };
    return [
      { label: POINT_TEXTS.allLayers, radio: true, checked: kept.layer === null, run: pick(null) },
      ...named.map((l) => ({ label: `${l.name} (${counts.get(l.id)})`, radio: true, checked: kept.layer === l.name, run: pick(l.name) })),
    ];
  }

  private refresh(): void {
    if (!this.el.isConnected) return;
    const { doc, selection } = this.ctx;
    this.points = [...doc.all()].filter((e: Entity): e is PointEntity => e.kind === 'point');
    const selected = selection.ids.value;
    const name = (id: string) => doc.layers.get(id)?.name ?? id;
    // A layer chosen that no longer holds points shows all.
    if (kept.layer !== null && !this.points.some((e) => name(e.layerId) === kept.layer)) kept.layer = null;
    const data = this.points.map((e) => rowOf(e, name(e.layerId), selected.has(e.id)));
    this.shown = pointTable(data, kept);
    this.ids = this.shown.map((i) => this.points[i].id);
    this.layerPick.set(kept.layer ?? POINT_TEXTS.allLayers);
    this.onlyBox.checked = kept.onlySelected;
    this.showBtn.disabled = selection.size === 0;
    this.count.textContent = `${this.shown.length} / ${this.points.length} nokta`;
    this.heads.forEach((th, i) => {
      const sorted = kept.sort !== null && POINT_COLUMNS[i].sort === kept.sort;
      th.setAttribute('aria-sort', sorted ? (kept.descending ? 'descending' : 'ascending') : 'none');
      th.classList.toggle('is-sorted', sorted);
      th.querySelector('.ptable__arrow')?.replaceChildren(sorted ? icon(kept.descending ? 'chevronDown' : 'chevronUp', 12) : '');
    });
    this.empty.hidden = this.shown.length > 0;
    replaceChildren(this.empty, this.points.length ? POINT_TEXTS.noMatch : POINT_TEXTS.none);
    if (this.anchor !== null && this.anchor >= this.shown.length) this.anchor = null;
    this.rows.set(this.shown.length);
    // The selection shows: when its first row changes it is scrolled into view, once (scrolling away stays), as the
    // desktop's table reveals it.
    const first = this.ids.findIndex((id) => selected.has(id));
    const key = first >= 0 ? `${this.ids[first]}@${first}` : null;
    if (key !== this.revealed) {
      this.revealed = key;
      if (first >= 0) this.rows.reveal(first);
    }
  }

  private row(i: number): HTMLElement {
    const e = this.points[this.shown[i]];
    const { format: f, doc, selection } = this.ctx;
    const cells = [String(i + 1), e.label ?? '', f.coord(e.p.x), f.coord(e.p.y), e.z !== undefined ? f.length(e.z, false) : '', e.attrs.Kod ?? '', doc.layers.get(e.layerId)?.name ?? ''];
    const on = selection.has(e.id);
    return h(
      'tr',
      { class: on ? 'is-selected' : null, 'data-at': String(i), 'aria-selected': String(on) },
      cells.map((c, j) => h('td', { class: POINT_COLUMNS[j].numeric ? 'num' : null }, c)),
    );
  }
}
