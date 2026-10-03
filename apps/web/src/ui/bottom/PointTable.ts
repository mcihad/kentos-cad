import type { AppContext } from '../../app/context';
import { listen } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import type { Entity, PointEntity } from '../../model/entities';
import { pointTable, type SortColumn, type TableQuery, type TableRow } from '../../model/ops/pointEditor';
import { Component } from '../Component';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { Dropdown } from '../widgets/Dropdown';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';
import { tooltip } from '../widgets/tooltip';
import { tableSpacer, VirtualRows } from '../widgets/VirtualRows';
import { COORD_FILES } from '../io/coordFiles';
import { batchTargets, importTargets, type BatchOp } from './pointBatch';
import { cellText, EDIT_COLUMNS, emptyDraft, nextCell, writeCell, writeDraft, type Draft, type EditColumn, type Outcome } from './pointEdit';
import { UNIT_PER_METRE } from '../../model/projectSettings';

/**
 * Noktalar, the bottom panel's point editor (docs/adr/0153 §1–§4): every point of the drawing in a table, its search,
 * layer and selection filters and its column sort the shared core's (`pointTable`, the same order the desktop shows).
 * A click selects the point in the drawing (Ctrl turns one over, Shift takes the run from the last click); the
 * drawing's selection shows in the rows, the first selected row scrolled into view when it changes (once: scrolling
 * away stays). Göster, or a double click on a row's number, zooms to the selected points.
 *
 * A double click on Ad, Y, X, Z or Kod edits it in place (./pointEdit.ts writes it): Enter writes and goes down the
 * column, Tab right, Shift+Tab left, a click elsewhere writes and stops, Esc gives up. Satır ekle opens a draft row at
 * the bottom; Enter writes it and opens the next, its name one more. Sil deletes the selected points.
 *
 * İşlemler ▾ and a row's right-click menu hold the batch operations (§5; ./pointBatch.ts, their windows
 * ./PointBatchDialog.ts, loaded on first use) over the selected rows, or every row when none is selected. A right
 * click leaves the selection as it is; on a row not selected its menu is that row's (its header names it), and a
 * command chosen there selects it alone first. Dışa aktar writes the same rows, in the table's order, through the
 * coordinate list window; İçe aktar (İşlemler ▾ only) reads one, then opens Çift noktaları ayıkla by Aynı ad over the
 * points whose names the file brought again. The desktop's is `apps/desktop/src/points/`.
 */

/** The columns: their header, the core's sort key (none: Sıra, the drawing's order), whether they hold numbers, the cell edited. */
export const POINT_COLUMNS: readonly { label: string; sort: SortColumn | null; numeric: boolean; edit: EditColumn | null }[] = [
  { label: 'Sıra', sort: null, numeric: true, edit: null },
  { label: 'Ad', sort: 'name', numeric: false, edit: 'name' },
  { label: 'Y (sağa)', sort: 'east', numeric: true, edit: 'east' },
  { label: 'X (yukarı)', sort: 'north', numeric: true, edit: 'north' },
  { label: 'Z (kot)', sort: 'z', numeric: true, edit: 'z' },
  { label: 'Kod', sort: 'code', numeric: false, edit: 'code' },
  { label: 'Katman', sort: 'layer', numeric: false, edit: null },
];

export const POINT_TEXTS = {
  search: 'Ad ya da kod ara',
  searchHint: 'Adda ya da kodda arar; * herhangi bir dizi: P1*, *0',
  allLayers: 'Bütün katmanlar',
  onlySelected: 'Yalnız seçililer',
  follow: 'Bağlı çizgiler izler',
  followHint: 'Nokta taşınınca ya da kotu değişince, o yerde köşesi olan çizgi, çoklu çizgi ve alanların köşeleri de izler.',
  add: 'Satır ekle',
  addHint: 'Tablonun sonunda yeni satır: Ad, Y, X, Z ve Kod yazılır, Enter etkin katmana yazar ve sonrakini açar.',
  remove: 'Sil',
  removeHint: 'Seçili noktaları siler',
  show: 'Göster',
  showHint: 'Seçili noktalara yakınlaştırır',
  actions: 'İşlemler',
  actionsHint: 'Seçili satırlara, seçim yoksa tablodaki bütün satırlara: Yeniden adlandır, Sıralı numara ver, Katmana taşı, Çift noktaları ayıkla, Dışa aktar. İçe aktar koordinat listesinden nokta alır.',
  rename: 'Yeniden adlandır…',
  renameHint: 'Adların başına önek ekler ya da baştaki öneki kaldırır',
  number: 'Sıralı numara ver…',
  numberHint: 'Tablodaki sırayla birer artan adlar verir',
  layer: 'Katmana taşı…',
  layerHint: 'Noktaları seçilen katmana taşır',
  dedupe: 'Çift noktaları ayıkla…',
  dedupeHint: 'Aynı adlı ya da aynı yerdeki noktalardan birini tutar, ötekileri siler',
  exportList: 'Dışa aktar…',
  exportHint: 'Noktaları tablonun sırasıyla koordinat listesi olarak yazar (NCN, TXT, CSV)',
  importList: 'İçe aktar…',
  importHint: 'Koordinat listesinden nokta alır; adları çizimde de varsa Çift noktaları ayıkla Aynı ad ile açılır',
  groupsHint: 'Tablo çift noktaların gruplarını gösteriyor; Sıra grubun numarasıdır. Süzgeci kaldırmak için tıklayın.',
  draft: 'Yeni',
  none: 'Çizimde nokta yok. Nokta aracıyla, Satır ekle ile ya da İşlemler ▾ › İçe aktar ile ekleyin.',
  noMatch: 'Süzgece uyan nokta yok.',
} as const;

/** The table's query and Bağlı çizgiler izler, kept for the session: leaving the tab and coming back keeps them. */
const kept: TableQuery & { follow: boolean } = { search: '', layer: null, onlySelected: false, sort: null, descending: false, follow: true };

/** The query as a new session starts it (the pictures start each scene from it). */
export function resetPointTable(): void {
  Object.assign(kept, { search: '', layer: null, onlySelected: false, sort: null, descending: false, follow: true });
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

/** A row's menu's header when the row is not selected: the point by its name. */
export function rowHeader(e: PointEntity): string {
  const name = e.label?.trim();
  return name ? `Nokta ${name}` : 'Adsız nokta';
}

/** The cell edited: a point's (by id) or the draft's. */
type Editing = { id: number | 'draft'; col: EditColumn };

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
  private readonly followBox: HTMLInputElement;
  private readonly showBtn: HTMLButtonElement;
  private readonly removeBtn: HTMLButtonElement;
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
  private editing: Editing | null = null;
  /** What was typed in the cell edited, kept when its value was refused (the cell opens again with it). */
  private typed: string | null = null;
  /** The open editor's field, while it is on screen; its blur writes unless a key already did. */
  private field: HTMLInputElement | null = null;
  private draft: Draft | null = null;
  /** Çiftleri göster: the groups shown (their members' ids), and each row's group number. */
  private groups: number[][] | null = null;
  private groupNo: number[] = [];
  private readonly groupsChip: HTMLButtonElement;

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
    this.layerPick = new Dropdown({ ariaLabel: 'Katman', width: 170, className: 'ptable__layer', items: () => this.layerItems() });
    this.onlyBox = h('input', { type: 'checkbox', checked: kept.onlySelected });
    this.d.add(
      listen(this.onlyBox, 'change', () => {
        kept.onlySelected = this.onlyBox.checked;
        this.schedule();
      }),
    );
    this.followBox = h('input', { type: 'checkbox', checked: kept.follow });
    this.d.add(listen(this.followBox, 'change', () => (kept.follow = this.followBox.checked)));
    const follow = h('label', { class: 'io-check ptable__only' }, this.followBox, POINT_TEXTS.follow);
    this.d.add(tooltip(follow, () => ({ title: POINT_TEXTS.follow, description: POINT_TEXTS.followHint }), 'top'));
    const button = (iconName: string, text: string, hint: string, run: () => void) => {
      const b = h('button', { class: 'btn btn--small ptable__btn', type: 'button' }, icon(iconName, 14), h('span', null, text));
      this.d.add(listen(b, 'click', run));
      this.d.add(tooltip(b, () => ({ title: text, description: hint }), 'top'));
      return b;
    };
    const add = button('plus', POINT_TEXTS.add, POINT_TEXTS.addHint, () => this.addRow());
    const actions = button('processing', POINT_TEXTS.actions, POINT_TEXTS.actionsHint, () => {
      const r = actions.getBoundingClientRect();
      const read: MenuItem = { label: POINT_TEXTS.importList, icon: 'import', detail: POINT_TEXTS.importHint, run: () => this.importList() };
      PopupMenu.open([{ kind: 'header', label: this.targets().header }, ...this.batchItems(), read], r, { owner: actions });
    });
    actions.append(icon('chevronDown', 12));
    this.removeBtn = button('erase', POINT_TEXTS.remove, POINT_TEXTS.removeHint, () => ctx.commands.execute('tool.erase'));
    this.showBtn = button('zoomSelection', POINT_TEXTS.show, POINT_TEXTS.showHint, () => ctx.commands.execute('view.zoomSelection'));
    this.groupsChip = h('button', { class: 'ptable__chip', type: 'button', hidden: true });
    this.d.add(listen(this.groupsChip, 'click', () => this.showGroups(null)));
    this.d.add(tooltip(this.groupsChip, () => ({ title: this.groupsChip.textContent ?? '', description: POINT_TEXTS.groupsHint }), 'top'));

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
      // Two groups: the filters on the left, the count and the buttons on the right; a narrow panel takes the right
      // group to a second line rather than cut it (the desktop's bar does the same).
      h(
        'div',
        { class: 'ptable__bar' },
        h('div', { class: 'ptable__group' }, search, this.layerPick.el, h('label', { class: 'io-check ptable__only' }, this.onlyBox, POINT_TEXTS.onlySelected), follow, this.groupsChip),
        h('div', { class: 'ptable__group ptable__group--end' }, this.count, actions, add, this.removeBtn, this.showBtn),
      ),
      this.scroller,
    );
    this.rows = new VirtualRows({ parent: this.body, scroller: this.scroller, row: (i) => this.row(i), spacer: tableSpacer(POINT_COLUMNS.length) });
    this.d.add(() => this.rows.dispose());

    // A click selects; a double click on the number zooms to it, on a value edits it.
    this.d.add(
      listen<MouseEvent>(this.body, 'click', (e) => {
        const at = this.rowAt(e);
        if (at === null || at >= this.ids.length || (e.target as HTMLElement).closest('input')) return;
        const how = { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey };
        const ids = clickPick(ctx.selection.ids.value, this.ids, at, this.anchor, how);
        if (!how.shift) this.anchor = at;
        ctx.selection.set(ids);
      }),
    );
    this.d.add(
      listen<MouseEvent>(this.body, 'dblclick', (e) => {
        const at = this.rowAt(e);
        const td = (e.target as HTMLElement).closest('td');
        if (at === null || !td || td.querySelector('input')) return;
        const col = POINT_COLUMNS[td.cellIndex]?.edit ?? null;
        if (at >= this.ids.length) {
          if (col && this.draft) this.edit({ id: 'draft', col });
          return;
        }
        if (td.cellIndex === 0) {
          ctx.selection.set([this.ids[at]]);
          ctx.commands.execute('view.zoomSelection');
        } else if (col) this.edit({ id: this.ids[at], col });
      }),
    );
    // A right click: the row's menu (the desktop's table opens it the same way, the selection as it is).
    this.d.add(
      listen<MouseEvent>(this.body, 'contextmenu', (e) => {
        const at = this.rowAt(e);
        if (at === null || at >= this.ids.length || (e.target as HTMLElement).closest('input')) return;
        e.preventDefault();
        PopupMenu.open(this.rowItems(at), { x: e.clientX, y: e.clientY }, { placement: 'point' });
      }),
    );
    this.d.add(watchAll([ctx.selection.ids, ctx.format.changed], () => this.schedule()));
    this.d.add(ctx.doc.events.on('changed', () => this.schedule()));
    this.d.add(ctx.doc.layers.version.subscribe(() => this.schedule()));
    // Built once the table is in the page (the first row's height is measured there).
    queueMicrotask(() => this.refresh());
  }

  /** The row index (in the order shown, the draft last) under a mouse event, or null. */
  private rowAt(e: MouseEvent): number | null {
    const tr = (e.target as HTMLElement).closest<HTMLElement>('tr[data-at]');
    return tr ? Number(tr.dataset.at) : null;
  }

  /** The rows the batch operations take now, and the header naming them. */
  private targets(): { ids: number[]; header: string } {
    return batchTargets(this.ids, (id) => this.ctx.selection.has(id));
  }

  /** The batch operations as a menu lists them, each run after `first` (a row's menu selects its row). */
  private batchItems(first: () => void = () => {}): MenuItem[] {
    const item = (kind: BatchOp['kind'], label: string, glyph: string, detail: string): MenuItem => ({
      label,
      icon: glyph,
      detail,
      disabled: this.ids.length === 0,
      run: () => (first(), this.openBatch(kind)),
    });
    return [
      item('rename', POINT_TEXTS.rename, 'pointRename', POINT_TEXTS.renameHint),
      item('number', POINT_TEXTS.number, 'pointNumber', POINT_TEXTS.numberHint),
      item('layer', POINT_TEXTS.layer, 'pointLayer', POINT_TEXTS.layerHint),
      item('dedupe', POINT_TEXTS.dedupe, 'pointDedupe', POINT_TEXTS.dedupeHint),
      { kind: 'separator' },
      { label: POINT_TEXTS.exportList, icon: 'export', detail: POINT_TEXTS.exportHint, disabled: this.ids.length === 0, run: () => (first(), this.exportList()) },
    ];
  }

  /**
   * A row's menu (docs/adr/0153 §5): Göster, the batch operations, Sil, over the selected rows when the row is one of
   * them; else over that row, its header naming it, a command selecting it alone first.
   */
  private rowItems(at: number): MenuItem[] {
    const { ctx } = this;
    const id = this.ids[at];
    const own = !ctx.selection.has(id);
    const first = () => {
      if (!ctx.selection.has(id)) {
        ctx.selection.set([id]);
        this.anchor = at;
      }
    };
    const header = own ? rowHeader(this.points[this.shown[at]]) : this.targets().header;
    return [
      { kind: 'header', label: header },
      { label: POINT_TEXTS.show, icon: 'zoomSelection', run: () => (first(), ctx.commands.execute('view.zoomSelection')) },
      { kind: 'separator' },
      ...this.batchItems(first),
      { kind: 'separator' },
      { label: POINT_TEXTS.remove, icon: 'erase', run: () => (first(), ctx.commands.execute('tool.erase')) },
    ];
  }

  /** The operation's window over the target rows, or `targets` (loaded on first use; a failed load says so). */
  private openBatch(kind: BatchOp['kind'], targets = this.targets(), imported = false): void {
    const { ids, header } = targets;
    import('./PointBatchDialog').then(
      (m) =>
        m.openPointBatchDialog(this.ctx, kind, ids, header, {
          follow: kept.follow,
          showGroups: (groups) => this.showGroups(groups),
          done: () => this.showGroups(null),
          ...(imported ? { by: 'name' as const, imported } : {}),
        }),
      (e: Error) => this.loadFailed(e),
    );
  }

  private loadFailed(e: Error): void {
    this.ctx.log.error(`Pencere yüklenemedi: ${e.message}. Bağlantıyı denetleyip yeniden deneyin.`);
  }

  /** Dışa aktar (docs/adr/0153 §5): the coordinate list window over the target rows, in the table's order. */
  private exportList(): void {
    const { ids } = this.targets();
    const selected = this.ids.some((id) => this.ctx.selection.has(id));
    import('../io/CoordExportDialog').then(
      (m) => m.openCoordExport(this.ctx, { table: { ids, selected } }),
      (e: Error) => this.loadFailed(e),
    );
  }

  /**
   * İçe aktar (docs/adr/0153 §5): the coordinate list window. The points go in even where their names are taken; then
   * Çift noktaları ayıkla opens by Aynı ad over the points carrying those names (İlki keeps the drawing's, Sonuncusu
   * the file's). The file is picked at the click itself (the browser's dialog needs the gesture), the window loading
   * alongside.
   */
  private importList(): void {
    const { ctx } = this;
    const file = ctx.files.pickForImport(COORD_FILES);
    const ui = import('../io/CoordImportDialog');
    void Promise.all([file, ui]).then(
      ([f, m]) => f && m.openCoordImport(ctx, f, COORD_FILES, { imported: (ids) => this.afterImport(ids) }),
      (e: Error) => this.loadFailed(e),
    );
  }

  private afterImport(imported: readonly number[]): void {
    const t = importTargets(this.ctx.doc, imported);
    if (t) this.openBatch('dedupe', t, true);
  }

  /** Çiftleri göster (docs/adr/0153 §5): the table shows only the groups, group by group; none shows the query again. */
  private showGroups(groups: number[][] | null): void {
    this.groups = groups?.length ? groups : null;
    this.schedule();
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
    // In the layer list's order.
    const order = layers.leaves().map((l) => l.id);
    const rank = (id: string) => (order.indexOf(id) + 1 || Infinity);
    const named = [...counts.keys()].sort((a, b) => rank(a) - rank(b)).map((id) => ({ id, name: layers.get(id)?.name ?? id }));
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
    if (this.groups) {
      // The groups as found, their points still in the drawing; the query waits.
      const at = new Map(this.points.map((e, i) => [e.id, i]));
      this.shown = [];
      this.groupNo = [];
      this.groups.forEach((g, n) =>
        g.forEach((id) => {
          const i = at.get(id);
          if (i !== undefined) (this.shown.push(i), this.groupNo.push(n + 1));
        }),
      );
    } else {
      this.shown = pointTable(data, kept);
      this.groupNo = [];
    }
    this.ids = this.shown.map((i) => this.points[i].id);
    this.groupsChip.hidden = !this.groups;
    if (this.groups) replaceChildren(this.groupsChip, `Çiftler: ${this.groups.length} grup`, icon('close', 12));
    // A point edited that went away (an undo) closes its editor.
    if (typeof this.editing?.id === 'number' && !this.ids.includes(this.editing.id)) this.editing = null;
    this.layerPick.set(kept.layer ?? POINT_TEXTS.allLayers);
    this.onlyBox.checked = kept.onlySelected;
    this.followBox.checked = kept.follow;
    this.showBtn.disabled = selection.size === 0;
    this.removeBtn.disabled = selection.size === 0;
    this.count.textContent = `${this.shown.length} / ${this.points.length} nokta`;
    this.heads.forEach((th, i) => {
      // East and north as the project's type names them (docs/adr/0165 §4).
      const name = th.querySelector('.ptable__sort > span');
      if (name) name.textContent = this.ctx.format.axesText(POINT_COLUMNS[i].label);
      const sorted = kept.sort !== null && POINT_COLUMNS[i].sort === kept.sort;
      th.setAttribute('aria-sort', sorted ? (kept.descending ? 'descending' : 'ascending') : 'none');
      th.classList.toggle('is-sorted', sorted);
      th.querySelector('.ptable__arrow')?.replaceChildren(sorted ? icon(kept.descending ? 'chevronDown' : 'chevronUp', 12) : '');
    });
    this.empty.hidden = this.shown.length > 0 || this.draft !== null;
    replaceChildren(this.empty, this.points.length ? POINT_TEXTS.noMatch : POINT_TEXTS.none);
    if (this.anchor !== null && this.anchor >= this.shown.length) this.anchor = null;
    // The rows go: a blur of the editor they hold is no click elsewhere.
    this.field = null;
    this.rows.set(this.shown.length + (this.draft ? 1 : 0));
    const editing = this.editing;
    if (editing) {
      const at = editing.id === 'draft' ? this.shown.length : this.ids.indexOf(editing.id);
      if (at >= 0) this.rows.reveal(at);
    }
    // The selection shows: when its first row changes it is scrolled into view, once (scrolling away stays), as the
    // desktop's table reveals it.
    const first = this.ids.findIndex((id) => selected.has(id));
    const key = first >= 0 ? `${this.ids[first]}@${first}` : null;
    if (key !== this.revealed) {
      this.revealed = key;
      if (first >= 0 && !editing) this.rows.reveal(first);
    }
  }

  private row(i: number): HTMLElement {
    if (i >= this.shown.length) return this.draftRow(i);
    const e = this.points[this.shown[i]];
    const { format: f, doc, selection } = this.ctx;
    // Under Çiftleri göster, Sıra is the group's number.
    const no = this.groups ? this.groupNo[i] : i + 1;
    const cells = [String(no), e.label ?? '', f.coord(e.p.x), f.coord(e.p.y), e.z !== undefined ? f.length(e.z, false) : '', e.attrs.Kod ?? '', doc.layers.get(e.layerId)?.name ?? ''];
    const on = selection.has(e.id);
    const ed = this.editing?.id === e.id ? this.editing.col : null;
    return h(
      'tr',
      { class: on ? 'is-selected' : null, 'data-at': String(i), 'aria-selected': String(on) },
      cells.map((c, j) => {
        const col = POINT_COLUMNS[j];
        const editing = col.edit !== null && col.edit === ed;
        return h('td', { class: `${col.numeric ? 'num' : ''}${editing ? ' is-editing' : ''}` || null }, editing ? this.editor(this.typed ?? cellText(e, col.edit!, UNIT_PER_METRE[this.ctx.doc.settings.unit]), col.numeric) : c);
      }),
    );
  }

  /** Satır ekle's row: its cells as typed, the one edited open. */
  private draftRow(i: number): HTMLElement {
    const d = this.draft ?? emptyDraft();
    const ed = this.editing?.id === 'draft' ? this.editing.col : null;
    const text: Record<EditColumn, string> = { name: d.name, east: d.east, north: d.north, z: d.z, code: d.code };
    return h(
      'tr',
      { class: 'ptable__draft', 'data-at': String(i) },
      POINT_COLUMNS.map((col, j) => {
        const editing = col.edit !== null && col.edit === ed;
        const value = j === 0 ? POINT_TEXTS.draft : col.edit ? text[col.edit] : '';
        return h('td', { class: `${col.numeric ? 'num' : ''}${editing ? ' is-editing' : ''}` || null }, editing ? this.editor(value, col.numeric) : value);
      }),
    );
  }

  /** The editor in a cell: the value in it chosen; its keys and its blur end the edit. */
  private editor(value: string, numeric: boolean): HTMLInputElement {
    const input = h('input', { class: `ptable__edit${numeric ? ' num' : ''}`, value, spellcheck: 'false', 'aria-label': 'Değer' });
    this.field = input;
    input.addEventListener('keydown', (e) => {
      e.stopPropagation();
      if (e.key === 'Enter') {
        e.preventDefault();
        this.finish(input.value, 'down');
      } else if (e.key === 'Tab') {
        e.preventDefault();
        this.finish(input.value, e.shiftKey ? 'left' : 'right');
      } else if (e.key === 'Escape') {
        e.preventDefault();
        this.cancel();
      }
    });
    input.addEventListener('blur', () => {
      // A key that ended the edit has taken the field away already.
      if (this.field === input) this.finish(input.value, null);
    });
    queueMicrotask(() => {
      if (!input.isConnected) return;
      input.focus({ preventScroll: true });
      input.select();
    });
    return input;
  }

  /** The cell edited from now (none: no editor); `typed`, what it opens with when a value was refused. */
  private edit(to: Editing | null, typed: string | null = null): void {
    this.editing = to;
    this.typed = typed;
    this.refresh();
  }

  /** Satır ekle: a draft row at the bottom, its Ad open. */
  private addRow(): void {
    this.draft ??= emptyDraft();
    this.edit({ id: 'draft', col: 'name' });
  }

  private cancel(): void {
    this.field = null;
    if (this.editing?.id === 'draft') this.draft = null;
    this.edit(null);
    this.ctx.view.focus();
  }

  private say(out: Outcome): void {
    for (const line of out.said) this.ctx.log.warn(line);
  }

  /** The edit ends with `value`: written, then the editor goes `how` (Enter down, Tab right, Shift+Tab left; null stops). */
  private finish(value: string, how: 'down' | 'right' | 'left' | null): void {
    const editing = this.editing;
    this.field = null;
    if (!editing) return;
    const { doc, settings } = this.ctx;
    if (editing.id === 'draft') {
      const d = { ...(this.draft ?? emptyDraft()), [editing.col]: value };
      this.draft = d;
      if (how === 'down') {
        const out = writeDraft(doc, d, doc.layers.active.value, settings.color.value);
        this.say(out);
        // Written: the next row, its name one more, its Y open.
        if (out.next !== null) {
          this.draft = emptyDraft(out.next);
          return this.edit({ id: 'draft', col: 'east' });
        }
        return this.edit(editing);
      }
      if (how === null) return this.edit(null);
      const c = EDIT_COLUMNS.indexOf(editing.col) + (how === 'right' ? 1 : -1);
      return this.edit({ id: 'draft', col: EDIT_COLUMNS[(c + EDIT_COLUMNS.length) % EDIT_COLUMNS.length] });
    }
    const e = doc.get(editing.id);
    if (!e || e.kind !== 'point') return this.edit(null);
    // Where to go next, by the rows' order before the write (a sort may move the row).
    const next = how ? nextCell(this.ids, editing.id, editing.col, how) : null;
    const out = writeCell(doc, e, editing.col, value, kept.follow);
    this.say(out);
    // A value refused: the cell stays open with what was typed (a click elsewhere gives it up).
    if (out.stay) return this.edit(how ? editing : null, how ? value : null);
    this.edit(next);
    if (!next) this.ctx.view.focus();
  }
}
