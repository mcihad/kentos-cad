import type { AppContext } from '../../app/context';
import { listen } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import type { Entity } from '../../model/entities';
import { attributeFields, entityObjects } from '../../model/expression/builderObjects';
import { compileExpression, expressionError } from '../../model/expression/expression';
import { featureTableModel, columnTable, type FeatureTableModel } from '../../model/featureTable';
import { checkValue, displayValue, type LayerField } from '../../model/layerFields';
import { featureTable, type TableShow } from '../../model/ops/featureTable';
import { entitiesSet } from '../../product/entitiesSet';
import { Component } from '../Component';
import { h, replaceChildren } from '../dom';
import { builderButton } from '../expression/builderApi';
import { icon } from '../icons';
import { Dropdown } from '../widgets/Dropdown';
import type { MenuItem } from '../widgets/PopupMenu';
import { tooltip } from '../widgets/tooltip';
import { tableSpacer, VirtualRows } from '../widgets/VirtualRows';
import { clickPick } from './PointTable';
import { appVariables } from '../../app/expressionVariables';

/**
 * Öznitelik tablosu, the bottom panel's Tablo tab (docs/adr/0199 §4): a layer's objects in a table, their attributes
 * by the layer's fields (./model/featureTable.ts reads the cells, the shared core's `featureTable` shows and orders
 * them, the desktop shows the same). Katman ▾ picks the layer (at first the first selected object's, else the active
 * one); Ara looks in the cells as they show (`*` any run); Göster takes every object, the selected ones or the ones in
 * the view; İfade süzgeci keeps those an expression holds for (ε opens the İfade oluşturucu).
 *
 * A header click sorts (ascending, descending, the drawing's order); a row click selects (Ctrl turns one over, Shift
 * takes the run), the drawing's selection shows in the rows, a double click on Sıra zooms to the object. A double
 * click on a value edits it: a value list and yes or no in a list, anything else in a field; Enter writes it
 * (`cad.entities.set`, “Değiştir”), Tab writes and goes right, Esc gives up; a value its field refuses stays in the
 * cell with why. Alanlar… opens the layer's fields.
 */

export const FEATURE_TEXTS = {
  layer: 'Katman',
  search: 'Ara',
  searchHint: 'Hücrelerin gösterilen değerlerinde arar; * herhangi bir dizi: 10*, *köşe',
  show: 'Göster',
  all: 'Tümü',
  selected: 'Seçililer',
  inView: 'Görünümdekiler',
  filter: 'İfade süzgeci',
  filterHint: 'Yalnız koşulu sağlayan nesneler: Kat > 3 ve Kullanım = ’K’. Boşsa hepsi.',
  zoom: 'Seçime yakınlaş',
  zoomHint: 'Seçili nesnelere yakınlaştırır',
  fields: 'Alanlar…',
  fieldsHint: 'Katmanın alanları: adları, türleri, kuralları, varsayılanları ve değer listeleri',
  noLayers: 'Çizimde katman yok.',
  none: 'Katmanda nesne yok.',
  noMatch: 'Süzgece uyan nesne yok.',
  empty: '—',
  required: 'Zorunlu alan',
} as const;

/** Göster's choices, in the menu's order. */
const SHOWS: readonly { show: TableShow; label: string }[] = [
  { show: 'all', label: FEATURE_TEXTS.all },
  { show: 'selected', label: FEATURE_TEXTS.selected },
  { show: 'inView', label: FEATURE_TEXTS.inView },
];

/** The Tür column's sort key (no attribute is named so). */
const KIND_SORT = '\u0000tür';

/** The table's choices, kept for the session: leaving the tab and coming back keeps them. */
const kept: { layer: string | null; search: string; show: TableShow; filter: string; sort: string | null; descending: boolean } = {
  layer: null,
  search: '',
  show: 'all',
  filter: '',
  sort: null,
  descending: false,
};

/** The choices as a new session starts them (the pictures start each scene from them). */
export function resetFeatureTable(layer: string | null = null): void {
  Object.assign(kept, { layer, search: '', show: 'all', filter: '', sort: null, descending: false });
}

/** A header click: ascending, then descending, then the drawing's order. */
function nextSort(key: string): void {
  if (kept.sort === key && kept.descending) Object.assign(kept, { sort: null, descending: false });
  else Object.assign(kept, { sort: key, descending: kept.sort === key });
}

/** The cell edited: an object (by id) and an attribute column's index. */
type Editing = { id: number; col: number };

export class FeatureTable extends Component {
  readonly el: HTMLElement;
  private readonly ctx: AppContext;
  private readonly scroller: HTMLElement;
  private readonly head = h('tr');
  private readonly body = h('tbody');
  private readonly count = h('span', { class: 'ptable__count num' });
  private readonly empty = h('div', { class: 'empty empty--inline ptable__empty' });
  private readonly layerPick: Dropdown;
  private readonly showPick: Dropdown;
  private readonly filterBox: HTMLInputElement;
  private readonly zoomBtn: HTMLButtonElement;
  private readonly fieldsBtn: HTMLButtonElement;
  private readonly rows: VirtualRows;
  private model: FeatureTableModel = { columns: [], entities: [], rows: [], problems: new Map() };
  /** The rows shown, by their index in the model, and their ids, in the order shown. */
  private shown: number[] = [];
  private ids: number[] = [];
  private anchor: number | null = null;
  private revealed: string | null = null;
  private pending = false;
  private editing: Editing | null = null;
  private typed: string | null = null;
  private field: HTMLInputElement | HTMLSelectElement | null = null;
  /** The expression filter's message when it cannot be read. */
  private filterError: string | null = null;

  constructor(ctx: AppContext) {
    super();
    this.ctx = ctx;
    const search = h('input', { class: 'field field--search ptable__search', type: 'search', placeholder: FEATURE_TEXTS.search, 'aria-label': FEATURE_TEXTS.search, value: kept.search, spellcheck: 'false' });
    this.d.add(tooltip(search, () => ({ title: FEATURE_TEXTS.search, description: FEATURE_TEXTS.searchHint }), 'top'));
    this.d.add(listen(search, 'input', () => ((kept.search = search.value), this.schedule())));
    this.layerPick = new Dropdown({ ariaLabel: FEATURE_TEXTS.layer, label: FEATURE_TEXTS.layer, width: 210, className: 'ftable__layer', items: () => this.layerItems() });
    this.showPick = new Dropdown({ ariaLabel: FEATURE_TEXTS.show, label: FEATURE_TEXTS.show, width: 190, items: () => this.showItems() });
    this.filterBox = h('input', { class: 'field ftable__filter', type: 'text', placeholder: FEATURE_TEXTS.filter, 'aria-label': FEATURE_TEXTS.filter, value: kept.filter, spellcheck: 'false' });
    this.d.add(tooltip(this.filterBox, () => ({ title: FEATURE_TEXTS.filter, description: this.filterError ?? FEATURE_TEXTS.filterHint }), 'top'));
    this.d.add(listen(this.filterBox, 'input', () => ((kept.filter = this.filterBox.value), this.schedule())));
    const builder = builderButton({
      get: () => kept.filter,
      set: (v) => {
        kept.filter = v;
        this.filterBox.value = v;
        this.schedule();
      },
      fields: () => attributeFields(this.model.columns.filter((c) => c.key !== null).map((c) => ({ name: c.key! }))),
      objects: () =>
        entityObjects(this.model.entities, (id) => ctx.doc.layers.get(id)?.name ?? id, {
          geometry: { evaluateExpression: (...a) => ctx.view.evaluateExpression(...a), evaluateExpressionIn: (...a) => ctx.view.evaluateExpressionIn(...a) },
          context: { variables: appVariables(ctx) },
        }),
      context: `${FEATURE_TEXTS.filter}`,
      variables: () => appVariables(ctx),
      fail: (m) => ctx.log.error(m),
    });
    const button = (iconName: string, text: string, hint: string, run: () => void) => {
      const b = h('button', { class: 'btn btn--small ptable__btn', type: 'button' }, icon(iconName, 14), h('span', null, text));
      this.d.add(listen(b, 'click', run));
      this.d.add(tooltip(b, () => ({ title: text, description: hint }), 'top'));
      return b;
    };
    this.zoomBtn = button('zoomSelection', FEATURE_TEXTS.zoom, FEATURE_TEXTS.zoomHint, () => ctx.commands.execute('view.zoomSelection'));
    this.fieldsBtn = button('layerFields', FEATURE_TEXTS.fields, FEATURE_TEXTS.fieldsHint, () => this.openFields());

    this.scroller = h('div', { class: 'ptable__scroll' }, h('table', { class: 'ftable__table' }, h('thead', null, this.head), this.body), this.empty);
    this.el = h(
      'div',
      { class: 'ptable ftable' },
      h(
        'div',
        { class: 'ptable__bar' },
        h('div', { class: 'ptable__group' }, this.layerPick.el, search, this.showPick.el, h('span', { class: 'ftable__expr' }, this.filterBox, builder)),
        h('div', { class: 'ptable__group ptable__group--end' }, this.count, this.zoomBtn, this.fieldsBtn),
      ),
      this.scroller,
    );
    this.rows = new VirtualRows({ parent: this.body, scroller: this.scroller, row: (i) => this.row(i), spacer: tableSpacer(999) });
    this.d.add(() => this.rows.dispose());

    this.d.add(
      listen<MouseEvent>(this.body, 'click', (e) => {
        const at = this.rowAt(e);
        if (at === null || (e.target as HTMLElement).closest('input, select')) return;
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
        if (at === null || !td || td.querySelector('input, select')) return;
        if (td.cellIndex === 0) {
          ctx.selection.set([this.ids[at]]);
          ctx.commands.execute('view.zoomSelection');
        } else if (td.cellIndex >= 2) this.edit({ id: this.ids[at], col: td.cellIndex - 1 });
      }),
    );
    this.d.add(watchAll([ctx.selection.ids, ctx.format.changed], () => this.schedule()));
    this.d.add(ctx.doc.events.on('changed', () => this.schedule()));
    this.d.add(ctx.doc.events.on('attrs', () => this.schedule()));
    this.d.add(ctx.doc.layers.version.subscribe(() => this.schedule()));
    this.d.add(ctx.view.camera.changed.subscribe(() => kept.show === 'inView' && this.schedule()));
    queueMicrotask(() => this.refresh());
  }

  private rowAt(e: MouseEvent): number | null {
    const tr = (e.target as HTMLElement).closest<HTMLElement>('tr[data-at]');
    return tr ? Number(tr.dataset.at) : null;
  }

  private schedule(): void {
    if (this.pending) return;
    this.pending = true;
    queueMicrotask(() => {
      this.pending = false;
      this.refresh();
    });
  }

  /** The layer shown: the one chosen while it is a layer, else the first selected object's, else the active one. */
  private layerId(): string | null {
    const { doc, selection } = this.ctx;
    const isLayer = (id: string | null | undefined): id is string => !!id && doc.layers.get(id)?.type === 'layer';
    if (isLayer(kept.layer)) return kept.layer;
    const first = [...selection.ids.value].map((id) => doc.get(id)).find((e) => e);
    kept.layer = isLayer(first?.layerId) ? first.layerId : isLayer(doc.layers.active.value) ? doc.layers.active.value : (doc.layers.leaves()[0]?.id ?? null);
    return kept.layer;
  }

  private layerItems(): MenuItem[] {
    const { layers } = this.ctx.doc;
    const current = this.layerId();
    return layers.leaves().map((l) => ({
      // A filtered layer counts what passes its filter (docs/adr/0211 §1).
      label: `${layers.path(l.id)} (${this.ctx.view.geometry.filterCounts(l.id)?.passed ?? this.ctx.doc.byLayer(l.id).length})`,
      radio: true,
      checked: l.id === current,
      run: () => {
        kept.layer = l.id;
        kept.sort = null;
        this.editing = null;
        this.schedule();
      },
    }));
  }

  private showItems(): MenuItem[] {
    return SHOWS.map((s) => ({ label: s.label, radio: true, checked: kept.show === s.show, run: () => ((kept.show = s.show), this.schedule()) }));
  }

  /** Alanlar… for the layer shown (the window loads on first use). */
  private openFields(): void {
    const layer = this.layerId();
    if (!layer) return;
    import('../layers/LayerFieldsDialog').then(
      (m) => m.openLayerFields(this.ctx, layer),
      (e: Error) => this.ctx.log.error(`Pencere yüklenemedi: ${e.message}. Bağlantıyı denetleyip yeniden deneyin.`),
    );
  }

  /** Which objects the expression filter keeps (null: no filter); a filter that cannot be read keeps none and says why. */
  private passes(entities: readonly Entity[]): boolean[] | null {
    this.filterError = null;
    const source = kept.filter.trim();
    if (!source) return null;
    const { ctx } = this;
    // The filter is evaluated when asked: it reads the project's `@` values (docs/adr/0214 §2.3).
    const c = compileExpression(source, { variables: appVariables(ctx) });
    if (!c.ok) {
      this.filterError = expressionError(c);
      return entities.map(() => false);
    }
    const geometry = { evaluateExpression: ctx.view.evaluateExpression.bind(ctx.view), evaluateExpressionIn: ctx.view.evaluateExpressionIn.bind(ctx.view) };
    const col = c.expr.evaluateAll({ entities, layerName: (id) => ctx.doc.layers.get(id)?.name ?? id, geometry }, 'bool');
    return entities.map((_, i) => col.value(i) === true);
  }

  private refresh(): void {
    if (!this.el.isConnected) return;
    const { doc, selection, view } = this.ctx;
    const layer = this.layerId();
    const node = layer ? doc.layers.get(layer) : undefined;
    const entities = layer ? doc.byLayer(layer).filter((e) => this.ctx.view.geometry.filterShown(e.id)) : [];
    const visible = kept.show === 'inView' ? new Set(view.inBox(view.camera.visibleBounds())) : null;
    this.model = featureTableModel(node?.fields ?? [], entities, (id) => selection.has(id), (e) => !visible || visible.has(e.id), this.passes(entities));
    const { columns } = this.model;
    const sortAt = kept.sort === null ? null : kept.sort === KIND_SORT ? 0 : columns.findIndex((c) => c.key === kept.sort);
    if (sortAt === -1) Object.assign(kept, { sort: null, descending: false });
    this.shown = featureTable(columnTable(columns), this.model.rows, { search: kept.search, show: kept.show, sort: sortAt === -1 ? null : sortAt, descending: kept.descending });
    this.ids = this.shown.map((i) => entities[i].id);
    if (this.editing && !this.ids.includes(this.editing.id)) this.editing = null;
    this.layerPick.set(layer ? doc.layers.path(layer) : FEATURE_TEXTS.noLayers);
    this.showPick.set(SHOWS.find((s) => s.show === kept.show)?.label ?? FEATURE_TEXTS.all);
    this.filterBox.classList.toggle('is-invalid', this.filterError !== null);
    this.zoomBtn.disabled = selection.size === 0;
    this.fieldsBtn.disabled = !layer;
    this.count.textContent = `${this.shown.length} / ${entities.length}`;
    this.renderHead(sortAt === -1 ? null : sortAt);
    this.empty.hidden = this.shown.length > 0;
    replaceChildren(this.empty, !layer ? FEATURE_TEXTS.noLayers : entities.length ? FEATURE_TEXTS.noMatch : FEATURE_TEXTS.none);
    if (this.anchor !== null && this.anchor >= this.shown.length) this.anchor = null;
    this.field = null;
    this.rows.set(this.shown.length);
    if (this.editing) {
      const at = this.ids.indexOf(this.editing.id);
      if (at >= 0) this.rows.reveal(at);
    }
    const selected = selection.ids.value;
    const first = this.ids.findIndex((id) => selected.has(id));
    const key = first >= 0 ? `${this.ids[first]}@${first}` : null;
    if (key !== this.revealed) {
      this.revealed = key;
      if (first >= 0 && !this.editing) this.rows.reveal(first);
    }
  }

  /** The header: Sıra, then the columns, each a sort button; a required field marked. */
  private renderHead(sortAt: number | null): void {
    const cells = [h('th', { class: 'num', scope: 'col' }, h('button', { class: 'ptable__sort', type: 'button' }, h('span', null, 'Sıra'), h('span', { class: 'ptable__arrow' })))];
    cells[0].firstElementChild!.addEventListener('click', () => (Object.assign(kept, { sort: null, descending: false }), this.schedule()));
    this.model.columns.forEach((c, j) => {
      const sorted = sortAt === j;
      const label = h('span', null, c.label, c.field?.required ? h('span', { class: 'ftable__req', title: FEATURE_TEXTS.required }, ' *') : null);
      const btn = h('button', { class: 'ptable__sort', type: 'button' }, label, h('span', { class: 'ptable__arrow' }, sorted ? icon(kept.descending ? 'chevronDown' : 'chevronUp', 12) : ''));
      btn.addEventListener('click', () => (nextSort(c.key ?? KIND_SORT), this.schedule()));
      const th = h('th', { class: `${c.order === 'number' ? 'num' : ''}${sorted ? ' is-sorted' : ''}`.trim() || null, scope: 'col', 'aria-sort': sorted ? (kept.descending ? 'descending' : 'ascending') : 'none' }, btn);
      cells.push(th);
    });
    replaceChildren(this.head, ...cells);
  }

  private row(i: number): HTMLElement {
    const at = this.shown[i];
    const e = this.model.entities[at];
    const r = this.model.rows[at];
    const on = this.ctx.selection.has(e.id);
    const ed = this.editing?.id === e.id ? this.editing.col : null;
    return h(
      'tr',
      { class: on ? 'is-selected' : null, 'data-at': String(i), 'aria-selected': String(on) },
      h('td', { class: 'num' }, String(i + 1)),
      this.model.columns.map((c, j) => {
        const problem = this.model.problems.get(`${at}:${j}`) ?? null;
        const editing = ed === j;
        const td = h(
          'td',
          { class: `${c.order === 'number' ? 'num' : ''}${problem ? ' ftable__bad' : ''}${editing ? ' is-editing' : ''}`.trim() || null },
          editing ? this.editor(e, c.key!, c.field) : r.cells[j].shown,
        );
        if (problem && !editing) td.title = problem;
        return td;
      }),
    );
  }

  /** The editor of an attribute: a list for a value list and yes or no, else a field with the value as it reads. */
  private editor(e: Entity, key: string, field: LayerField | null): HTMLElement {
    const raw = Object.hasOwn(e.attrs, key) ? e.attrs[key] : '';
    const checked = field ? checkValue(field, raw) : null;
    const value = checked && 'value' in checked ? checked.value : raw;
    if (field && (field.values?.length || field.kind === 'boolean')) {
      const options = field.values?.length ? field.values.map((c) => ({ code: c.code, label: c.label })) : [{ code: 'true', label: 'Evet' }, { code: 'false', label: 'Hayır' }];
      const select = h(
        'select',
        { class: 'ptable__edit ftable__select', 'aria-label': field.alias ?? field.name },
        h('option', { value: '' }, FEATURE_TEXTS.empty),
        options.map((o) => h('option', { value: o.code, selected: o.code === value }, o.label)),
      );
      this.field = select;
      this.d.add(listen(select, 'change', () => this.finish(select.value, 'enter')));
      this.keys(select);
      queueMicrotask(() => select.isConnected && select.focus({ preventScroll: true }));
      return select;
    }
    const text = this.typed ?? (field && checked && 'value' in checked ? displayValue(field, value) : raw);
    const input = h('input', { class: `ptable__edit${field && (field.kind === 'integer' || field.kind === 'decimal') ? ' num' : ''}`, value: text, spellcheck: 'false', 'aria-label': field?.alias ?? key });
    this.field = input;
    this.keys(input);
    queueMicrotask(() => {
      if (!input.isConnected) return;
      input.focus({ preventScroll: true });
      input.select();
    });
    return input;
  }

  /** An editor's keys and blur: Enter writes, Tab writes and goes right (Shift+Tab left), Esc gives up, a blur writes. */
  private keys(el: HTMLInputElement | HTMLSelectElement): void {
    (el as HTMLElement).addEventListener('keydown', (e) => {
      e.stopPropagation();
      if (e.key === 'Enter') {
        e.preventDefault();
        this.finish(el.value, 'enter');
      } else if (e.key === 'Tab') {
        e.preventDefault();
        this.finish(el.value, e.shiftKey ? 'left' : 'right');
      } else if (e.key === 'Escape') {
        e.preventDefault();
        this.field = null;
        this.edit(null);
        this.ctx.view.focus();
      }
    });
    el.addEventListener('blur', () => {
      if (this.field === el) this.finish(el.value, 'blur');
    });
  }

  private edit(to: Editing | null, typed: string | null = null): void {
    this.editing = to;
    this.typed = typed;
    this.refresh();
  }

  /**
   * The edit ends with `value`: written (blank takes the attribute away); Tab goes to the next value right (Shift+Tab
   * left), Enter and a blur stop. A value refused stays in its open cell with what was typed, but for a blur (a click
   * elsewhere gives it up).
   */
  private finish(value: string, how: 'enter' | 'right' | 'left' | 'blur'): void {
    const editing = this.editing;
    this.field = null;
    if (!editing) return;
    const { doc, log } = this.ctx;
    const column = this.model.columns[editing.col];
    const uid = doc.uidOf(editing.id);
    if (!column?.key || !uid) return this.edit(null);
    const r = entitiesSet.execute({ doc }, { uids: [uid], attrs: { [column.key]: value.trim() ? value : null }, operation: 'attributes' });
    if (r.status !== 'completed') {
      if ('error' in r) log.warn(r.error.message);
      return how === 'blur' ? this.edit(null) : this.edit(editing, value);
    }
    const last = this.model.columns.length - 1;
    const col = how === 'right' ? editing.col + 1 : how === 'left' ? editing.col - 1 : null;
    this.edit(col !== null && col >= 1 && col <= last ? { id: editing.id, col } : null);
    if (col === null && how !== 'blur') this.ctx.view.focus();
  }
}
