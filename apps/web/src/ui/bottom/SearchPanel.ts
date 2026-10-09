import type { AppContext } from '../../app/context';
import { listen } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import type { Vec2 } from '../../model/geometry';
import { attributeNames, inScope, layerCounts, searchIndex, type SearchIndex } from '../../model/dataSearch';
import { dataSearch, type SearchFound } from '../../model/ops/dataSearch';
import { Component } from '../Component';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { Dropdown } from '../widgets/Dropdown';
import type { MenuItem } from '../widgets/PopupMenu';
import { tooltip } from '../widgets/tooltip';
import { tableSpacer, VirtualRows } from '../widgets/VirtualRows';
import { clickPick } from './PointTable';
import { countText, fieldCell, fieldsOn, nextSort, placeOf, queryOf, SEARCH_COLUMNS, SEARCH_LIMIT, SEARCH_TEXTS as T, searchDefaults, type SearchState } from './searchPlan';

/**
 * Arama, the bottom panel's data search (docs/adr/0178): the search box over every object's label, text, block name
 * and attributes (the shared core's `dataSearch`: the same rows, order and cut on the desktop), the fields asked for,
 * the layer and the three options, and the objects found as rows (Katman, Tür, Alan, Değer). A click on a row selects
 * the object and zooms to it; Ctrl turns one over and Shift takes the run from the last click, neither zooming; Hepsini
 * seç selects and fits every object found, Göster zooms to the selected ones; Enter in the box takes the first row.
 * A coordinate typed in the box (Y,X, or X,Y in a CAD project) shows Git: the view comes to it and the place is marked
 * over the drawing (`selection.mark`), until İşareti kaldır, another place or another drawing. The desktop's is
 * `apps/desktop/src/search/`.
 */

/** The panel's choices, kept for the session: leaving the tab and coming back keeps them. */
const kept: SearchState = searchDefaults();

/** The choices as a new session starts them (the pictures start each scene from them). */
export function resetSearchPanel(): void {
  Object.assign(kept, searchDefaults());
}

/** The panel on screen, if any: the command that opens the tab gives its box the keyboard. */
let live: SearchPanel | null = null;

/** Veride ara (`data.search`): the keyboard to the search box, its words chosen. */
export function focusSearch(): void {
  live?.focusBox();
}

/** How long typing waits before the drawing is searched (ms): one search for a burst of keys. */
const TYPING = 120;

export class SearchPanel extends Component {
  readonly el: HTMLElement;
  private readonly ctx: AppContext;
  private readonly input: HTMLInputElement;
  private readonly layerPick: Dropdown;
  private readonly attrPick: Dropdown;
  private readonly chips: Record<'label' | 'text' | 'block' | 'attrs', HTMLButtonElement>;
  private readonly caseBox: HTMLInputElement;
  private readonly wholeBox: HTMLInputElement;
  private readonly onlyBox: HTMLInputElement;
  private readonly count = h('span', { class: 'ptable__count num dsearch__count' });
  private readonly selectBtn: HTMLButtonElement;
  private readonly showBtn: HTMLButtonElement;
  private readonly banner: HTMLElement;
  private readonly goGroup: HTMLElement;
  private readonly goText = h('span', { class: 'dsearch__place' });
  private readonly markGroup: HTMLElement;
  private readonly markText = h('span', { class: 'dsearch__place' });
  private readonly scroller: HTMLElement;
  private readonly body = h('tbody');
  private readonly heads: HTMLElement[] = [];
  private readonly empty = h('div', { class: 'empty empty--inline ptable__empty' });
  private readonly rows: VirtualRows;
  /** The drawing's objects with something to find, as of the last search. */
  private index: SearchIndex = { ids: [], layerIds: [], records: [] };
  /** What the index was made for: the drawing's revision, its layers' and its blocks'. */
  private builtFor: [number, number, unknown] | null = null;
  /** The index positions the scope took, and the rows the core answered for them. */
  private subset: number[] = [];
  private found: SearchFound = { rows: [], total: 0 };
  /** The ids of the objects shown, in the order shown (for clicks). */
  private ids: number[] = [];
  /** The last click without Shift, in the order shown. */
  private anchor: number | null = null;
  /** The first selected row last scrolled to (its id and place): a change of it scrolls, once. */
  private revealed: string | null = null;
  /** The place the box's words name, if they name one. */
  private place: Vec2 | null = null;
  private pending: 'search' | 'paint' | null = null;
  private timer = 0;

  constructor(ctx: AppContext) {
    super();
    this.ctx = ctx;
    live = this;
    this.d.add(() => {
      if (live === this) live = null;
    });

    this.input = h('input', { class: 'field field--search ptable__search dsearch__input', type: 'search', placeholder: ctx.format.axesText(T.placeholder), 'aria-label': T.search, value: kept.text, spellcheck: 'false', autocomplete: 'off' });
    this.d.add(tooltip(this.input, () => ({ title: T.search, shortcut: ctx.keymap.chordFor('data.search'), description: ctx.format.axesText(T.searchHint) }), 'top'));
    this.d.add(
      listen(this.input, 'input', () => {
        kept.text = this.input.value;
        this.place = placeOf(kept.text, (v) => ctx.format.toMetres(v));
        this.schedule('search', TYPING);
      }),
    );
    this.d.add(listen<KeyboardEvent>(this.input, 'keydown', (e) => this.onKey(e)));

    this.layerPick = new Dropdown({ ariaLabel: 'Katman', width: 180, className: 'ptable__layer', items: () => this.layerItems() });
    this.attrPick = new Dropdown({ ariaLabel: T.attrPick, width: 150, className: 'dsearch__attr', items: () => this.attrItems() });

    const chip = (field: 'label' | 'text' | 'block' | 'attrs', text: string, hint: string) => {
      const b = h('button', { class: 'dsearch__chip', type: 'button', 'aria-pressed': String(kept.fields[field]) }, text);
      this.d.add(
        listen(b, 'click', () => {
          kept.fields[field] = !kept.fields[field];
          this.schedule('search');
        }),
      );
      this.d.add(tooltip(b, () => ({ title: text, description: hint }), 'top'));
      return b;
    };
    this.chips = {
      label: chip('label', T.labelField, T.labelHint),
      text: chip('text', T.textField, T.textHint),
      block: chip('block', T.blockField, T.blockHint),
      attrs: chip('attrs', T.attrsField, T.attrsHint),
    };

    const box = (text: string, hint: string | null, on: boolean, set: (v: boolean) => void) => {
      const input = h('input', { type: 'checkbox', checked: on });
      this.d.add(
        listen(input, 'change', () => {
          set(input.checked);
          this.schedule('search');
        }),
      );
      const label = h('label', { class: 'io-check ptable__only' }, input, text);
      if (hint) this.d.add(tooltip(label, () => ({ title: text, description: hint }), 'top'));
      return input;
    };
    this.caseBox = box(T.matchCase, null, kept.matchCase, (v) => (kept.matchCase = v));
    this.wholeBox = box(T.wholeWord, T.wholeWordHint, kept.wholeWord, (v) => (kept.wholeWord = v));
    this.onlyBox = box(T.onlySelected, null, kept.onlySelected, (v) => (kept.onlySelected = v));

    const button = (iconName: string, text: string, hint: string, run: () => void, extra = '') => {
      const b = h('button', { class: `btn btn--small ptable__btn${extra}`, type: 'button' }, icon(iconName, 14), h('span', null, text));
      this.d.add(listen(b, 'click', run));
      this.d.add(tooltip(b, () => ({ title: text, description: hint }), 'top'));
      return b;
    };
    this.selectBtn = button('selectAll', T.selectAll, T.selectAllHint, () => this.selectEverything());
    this.showBtn = button('zoomSelection', T.show, T.showHint, () => ctx.commands.execute('view.zoomSelection'));

    // Koordinata git's bar: the place typed with Git, the place marked with İşareti kaldır.
    const go = button('target', T.go, T.goHint, () => this.goTo(), ' dsearch__go-btn');
    const unmark = button('markClear', T.unmark, T.unmarkHint, () => ctx.selection.mark.set(null));
    this.goGroup = h('div', { class: 'dsearch__bar-group' }, h('span', { class: 'dsearch__bar-label' }, T.coordinate), this.goText, go);
    this.markGroup = h('div', { class: 'dsearch__bar-group' }, h('span', { class: 'dsearch__bar-label' }, T.mark), this.markText, unmark);
    this.banner = h('div', { class: 'dsearch__banner' }, this.goGroup, this.markGroup);

    const head = h(
      'tr',
      null,
      SEARCH_COLUMNS.map((c, i) => {
        // Sıra is a number: its header sits on the right, as the point editor's.
        const th = h('th', { scope: 'col', class: `dsearch__th dsearch__th--${i}${i === 0 ? ' num' : ''}` }, h('button', { class: 'ptable__sort', type: 'button' }, h('span', null, c.label), h('span', { class: 'ptable__arrow' })));
        this.d.add(
          listen(th.firstElementChild as HTMLElement, 'click', () => {
            Object.assign(kept, nextSort(kept.sort, kept.descending, c.sort));
            this.schedule('search');
          }),
        );
        this.heads.push(th);
        return th;
      }),
    );
    this.scroller = h('div', { class: 'ptable__scroll' }, h('table', { class: 'dsearch__table' }, h('thead', null, head), this.body), this.empty);
    this.el = h(
      'div',
      { class: 'ptable dsearch' },
      // Two lines: the words, the layer and the count with its buttons; then the fields and the options (a narrow panel wraps the second).
      h(
        'div',
        { class: 'ptable__bar' },
        h('div', { class: 'ptable__group' }, this.input, this.layerPick.el),
        h('div', { class: 'ptable__group ptable__group--end' }, this.count, this.selectBtn, this.showBtn),
        h(
          'div',
          { class: 'dsearch__choices' },
          h('div', { class: 'ptable__group dsearch__fields', role: 'group', 'aria-label': 'Aranan alanlar' }, h('span', { class: 'dsearch__bar-label' }, T.fieldsLabel), this.chips.label, this.chips.text, this.chips.block, this.chips.attrs, this.attrPick.el),
          h('div', { class: 'ptable__group' }, this.caseBox.parentElement, this.wholeBox.parentElement, this.onlyBox.parentElement),
        ),
      ),
      this.banner,
      this.scroller,
    );
    this.rows = new VirtualRows({ parent: this.body, scroller: this.scroller, row: (i) => this.row(i), spacer: tableSpacer(SEARCH_COLUMNS.length) });
    this.d.add(() => this.rows.dispose());
    this.d.add(() => window.clearTimeout(this.timer));

    // A click selects the object and goes to it; Ctrl turns one over, Shift takes the run: neither goes.
    this.d.add(
      listen<MouseEvent>(this.body, 'click', (e) => {
        const tr = (e.target as HTMLElement).closest<HTMLElement>('tr[data-at]');
        if (!tr) return;
        const at = Number(tr.dataset.at);
        if (at >= this.ids.length) return;
        const how = { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey };
        if (how.ctrl || how.shift) {
          const ids = clickPick(ctx.selection.ids.value, this.ids, at, this.anchor, how);
          if (!how.shift) this.anchor = at;
          ctx.selection.set(ids);
        } else {
          this.anchor = at;
          this.pick(at);
        }
      }),
    );
    // What follows the drawing: its selection (a row's mark, Yalnız seçimde), its objects, layers and blocks, its marks.
    this.d.add(ctx.selection.ids.subscribe(() => this.schedule(kept.onlySelected ? 'search' : 'paint')));
    this.d.add(watchAll([ctx.format.changed], () => this.schedule('paint')));
    this.d.add(ctx.doc.events.on('changed', () => this.schedule('search')));
    this.d.add(ctx.doc.events.on('attrs', () => this.schedule('search')));
    this.d.add(ctx.doc.layers.version.subscribe(() => this.schedule('search')));
    this.d.add(ctx.doc.blocks.subscribe(() => this.schedule('search')));
    this.d.add(ctx.selection.mark.subscribe(() => this.schedule('paint')));
    this.place = placeOf(kept.text, (v) => ctx.format.toMetres(v));
    // Built once the table is in the page (the first row's height is measured there).
    queueMicrotask(() => this.refresh('search'));
  }

  /** The keyboard to the search box, its words chosen. */
  focusBox(): void {
    this.input.focus();
    this.input.select();
  }

  private onKey(e: KeyboardEvent): void {
    // Enter and Esc are the box's own; the drawing's shortcuts that work in a field (Ctrl+S, F2) go on to the keymap.
    if (e.key === 'Enter') {
      e.preventDefault();
      e.stopPropagation();
      // The words as typed, though their pause is not over.
      this.flush();
      if (this.place) this.goTo();
      else if (this.ids.length) this.pick(0);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      if (this.input.value) {
        this.input.value = '';
        kept.text = '';
        this.place = null;
        this.schedule('search');
      } else this.ctx.view.focus();
    }
  }

  /** Changes come in bursts (an undo changes many objects; keys are typed): one refresh after them. */
  private schedule(what: 'search' | 'paint', delay = 0): void {
    // A search is the stronger of the two.
    this.pending = what === 'search' || this.pending === 'search' ? 'search' : 'paint';
    window.clearTimeout(this.timer);
    this.timer = window.setTimeout(() => this.flush(), delay);
  }

  /** What waits for its pause is done now: a key that acts on the rows (Enter) acts on the words as typed. */
  private flush(): void {
    if (!this.pending) return;
    window.clearTimeout(this.timer);
    const next = this.pending;
    this.pending = null;
    this.refresh(next);
  }

  /** Select the row's object and zoom to it. */
  private pick(at: number): void {
    const id = this.ids[at];
    if (id === undefined) return;
    this.ctx.selection.set([id]);
    this.ctx.view.zoomToObjects([id]);
  }

  /** Hepsini seç: every object found (not only the rows listed), selected and fitted. */
  private selectEverything(): void {
    this.flush();
    const all = this.search(0);
    const ids = all.rows.map((r) => this.index.ids[this.subset[r.record]]);
    if (!ids.length) return;
    this.ctx.selection.set(ids);
    this.ctx.view.zoomToObjects(ids);
  }

  /** Git: the view comes to the place typed, which is marked over the drawing. */
  private goTo(): void {
    const p = this.place;
    if (!p) return;
    const { ctx } = this;
    ctx.selection.mark.set(p);
    ctx.view.centerOn(p);
    ctx.log.info(`Koordinata gidildi: ${ctx.format.point(p)}.`);
  }

  /** The layers that hold objects with something to find, in the layer list's order, each with its count. */
  private layerItems(): MenuItem[] {
    const pick = (id: string | null) => () => {
      kept.layer = id;
      this.schedule('search');
    };
    return [
      { label: T.allLayers, radio: true, checked: kept.layer === null, run: pick(null) },
      ...layerCounts(this.index, this.ctx.doc).map((l) => ({ label: `${l.name} (${l.count})`, radio: true, checked: kept.layer === l.id, run: pick(l.id) })),
    ];
  }

  /** Bütün öznitelikler, then the attribute names the drawing carries: the one searched. */
  private attrItems(): MenuItem[] {
    const pick = (name: string | null) => () => {
      kept.fields.attrName = name;
      this.schedule('search');
    };
    return [
      { label: T.allAttrs, radio: true, checked: kept.fields.attrName === null, run: pick(null) },
      ...attributeNames(this.index).map((n) => ({ label: n, radio: true, checked: kept.fields.attrName === n, run: pick(n) })),
    ];
  }

  /** The core's answer for the objects in the scope: the rows (`limit` of them, 0: all) and the count. */
  private search(limit: number): SearchFound {
    const { records } = this.index;
    return dataSearch(
      this.subset.map((i) => records[i]),
      queryOf(kept, limit),
    );
  }

  private refresh(what: 'search' | 'paint'): void {
    if (!this.el.isConnected) return;
    const { doc, selection, format } = this.ctx;
    if (what === 'search') {
      // The index is made again only when the drawing, its layers or its blocks changed.
      const key: [number, number, unknown] = [doc.revision, doc.layers.version.value, doc.blocks.value];
      if (!this.builtFor || this.builtFor[0] !== key[0] || this.builtFor[1] !== key[1] || this.builtFor[2] !== key[2]) {
        const index = searchIndex(doc);
        // A layer's filter leaves out what it does not pass (docs/adr/0211 §1).
        const shown = index.ids.map((id) => this.ctx.view.geometry.filterShown(id));
        this.index = shown.every(Boolean)
          ? index
          : { ids: index.ids.filter((_, i) => shown[i]), layerIds: index.layerIds.filter((_, i) => shown[i]), records: index.records.filter((_, i) => shown[i]) };
        this.builtFor = key;
      }
      // A layer chosen that holds nothing to find any more shows all.
      if (kept.layer !== null && !this.index.layerIds.includes(kept.layer)) kept.layer = null;
      if (kept.fields.attrName !== null && !attributeNames(this.index).includes(kept.fields.attrName)) kept.fields.attrName = null;
      this.subset = inScope(this.index, { layerId: kept.layer, selected: kept.onlySelected ? selection.ids.value : null });
      const asks = kept.text.trim() !== '' && fieldsOn(kept.fields);
      this.found = asks ? this.search(SEARCH_LIMIT) : { rows: [], total: 0 };
      this.ids = this.found.rows.map((r) => this.index.ids[this.subset[r.record]]);
      if (this.anchor !== null && this.anchor >= this.ids.length) this.anchor = null;
    }

    // The controls as the choices are; the coordinate order and the unit are the open drawing's (they change with it).
    this.input.value !== kept.text && (this.input.value = kept.text);
    this.input.placeholder = format.axesText(T.placeholder);
    this.place = placeOf(kept.text, (v) => format.toMetres(v));
    for (const [field, b] of Object.entries(this.chips)) b.setAttribute('aria-pressed', String(kept.fields[field as keyof typeof this.chips]));
    this.attrPick.el.disabled = !kept.fields.attrs;
    this.attrPick.set(kept.fields.attrName ?? T.allAttrs);
    this.layerPick.set(kept.layer === null ? T.allLayers : (doc.layers.get(kept.layer)?.name ?? T.allLayers));
    this.caseBox.checked = kept.matchCase;
    this.wholeBox.checked = kept.wholeWord;
    this.onlyBox.checked = kept.onlySelected;
    this.onlyBox.disabled = selection.size === 0 && !kept.onlySelected;
    this.selectBtn.disabled = this.found.total === 0;
    this.showBtn.disabled = selection.size === 0;
    this.count.textContent = this.found.total || kept.text.trim() ? countText(this.found.rows.length, this.found.total) : '';

    // Koordinata git's bar.
    const mark = selection.mark.value;
    this.goGroup.hidden = !this.place;
    if (this.place) this.goText.textContent = format.point(this.place);
    this.markGroup.hidden = !mark;
    if (mark) this.markText.textContent = format.point(mark);
    this.banner.hidden = !this.place && !mark;

    this.heads.forEach((th, i) => {
      const sorted = kept.sort !== null && SEARCH_COLUMNS[i].sort === kept.sort;
      th.setAttribute('aria-sort', sorted ? (kept.descending ? 'descending' : 'ascending') : 'none');
      th.classList.toggle('is-sorted', sorted);
      th.querySelector('.ptable__arrow')?.replaceChildren(sorted ? icon(kept.descending ? 'chevronDown' : 'chevronUp', 12) : '');
    });
    const word = kept.text.trim();
    this.empty.hidden = this.ids.length > 0;
    if (!this.ids.length) replaceChildren(this.empty, !word ? format.axesText(T.noQuery) : !fieldsOn(kept.fields) ? T.noFields : !this.index.records.length ? T.noData : T.noMatch(word));
    this.rows.set(this.ids.length);
    // The selection shows: when its first row changes it is scrolled into view, once (scrolling away stays).
    const selected = selection.ids.value;
    const first = this.ids.findIndex((id) => selected.has(id));
    const key = first >= 0 ? `${this.ids[first]}@${first}` : null;
    if (key !== this.revealed) {
      this.revealed = key;
      if (first >= 0) this.rows.reveal(first);
    }
  }

  private row(i: number): HTMLElement {
    const row = this.found.rows[i];
    const { doc, selection } = this.ctx;
    const id = this.ids[i];
    const record = this.index.records[this.subset[row.record]];
    const layerId = this.index.layerIds[this.subset[row.record]];
    const hidden = !doc.layers.isVisible(layerId);
    const locked = doc.layers.isLocked(layerId);
    const on = selection.has(id);
    const why = hidden ? 'Katman gizli' : locked ? 'Katman kilitli' : null;
    // A cell the column cuts says its whole self on hover; the layer's says why it is quiet too.
    const field = fieldCell(row);
    return h(
      'tr',
      { class: `${on ? 'is-selected' : ''}${hidden ? ' dsearch__row--quiet' : ''}` || null, 'data-at': String(i), 'aria-selected': String(on) },
      h('td', { class: 'num' }, String(i + 1)),
      h('td', { class: 'dsearch__layer', title: why ? `${why}: ${record.layer}` : record.layer }, hidden ? icon('eyeOff', 12) : locked ? icon('lock', 12) : null, h('span', null, record.layer)),
      h('td', { title: record.kind }, record.kind),
      h('td', { class: 'dsearch__field', title: field }, field),
      h('td', { class: 'dsearch__value', title: row.value }, row.value),
    );
  }
}
