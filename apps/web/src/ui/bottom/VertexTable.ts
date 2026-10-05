import type { AppContext } from '../../app/context';
import { listen } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import { entityArea, entityLength, type Entity } from '../../model/entities';
import { bearingGrad } from '../../model/geometry';
import type { Elevated } from '../../model/ops/elevation';
import type { VertexKind, VertexRow } from '../../model/ops/vertexTable';
import { UNIT_PER_METRE } from '../../model/projectSettings';
import { elevatedPaths } from '../../product/elevation';
import { Component } from '../Component';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { tooltip } from '../widgets/tooltip';
import { tableSpacer, VirtualRows } from '../widgets/VirtualRows';
import { neighboursOf } from '../../tools/neighbours';
import {
  cellEditable,
  cellText,
  emptyVertexDraft,
  isEditable,
  removeVertices,
  VERTEX_COLUMNS,
  vertexRows,
  writeVertexCell,
  writeVertexDraft,
  type At,
  type Editable,
  type Outcome,
  type VertexColumn,
  type VertexDraft,
} from './vertexEdit';

/**
 * Köşe tablosu (docs/adr/0172): the bottom panel's Koordinat listesi for one line, polyline or area, its vertices in
 * the core's order (the outer ring, its holes, then each part's ring and holes) with Halka (when there is more than
 * one), Y, X, Z, the signed radius of the edge leaving each, its chord and bearing; the object's measures below.
 *
 * A click selects a row (Ctrl turns one over, Shift takes the run from the last click); the selected rows' vertices
 * are ringed in the drawing. A double click on Köşe, or Göster, brings the vertex to the view's middle. While the
 * table edits the object (one selected, its layer not locked), a double click on Y, X, Z or Yarıçap edits it in place
 * (./vertexEdit.ts writes it): Enter writes and goes down the column, Tab right, Shift+Tab left, a click elsewhere
 * writes and stops, Esc gives up. Satır ekle opens a draft row under the last selected row (the last row when none
 * is): Y, X and Z typed, Enter adds the vertex after that row's and opens the next draft under it. Sil, or Delete in
 * the table, removes the selected rows' vertices. The desktop's is `apps/desktop/src/vertices/`.
 */

export const VERTEX_TEXTS = {
  show: 'Göster',
  showHint: 'Seçili satırın köşesini görünümün ortasına getirir',
  add: 'Satır ekle',
  addHint: 'Seçili satırın altında yeni satır: Y, X ve Z yazılır, Enter köşeyi o satırın köşesinden sonra ekler ve sonrakini açar.',
  remove: 'Sil',
  removeHint: 'Seçili satırların köşelerini siler (Delete)',
  draft: 'Yeni',
  hint: 'Değeri değiştirmek için hücreye çift tıklayın; Enter yazar, Tab sağa geçer.',
  locked: 'Katman kilitli; köşeler düzenlenmez.',
  many: 'Birden çok nesne seçili; düzenlemek için tek nesne seçin.',
  lockedNote: '(katman kilitli)',
  manyNote: '(ilk nesne gösteriliyor; düzenlemek için tek nesne seçin)',
} as const;

/** A column: its header, whether it holds numbers, the cell edited. */
interface Col {
  key: 'no' | 'ring' | VertexColumn | 'chord' | 'bearing';
  label: string;
  numeric: boolean;
  edit: VertexColumn | null;
}

/** The rings' names, path by path (`elevatedPaths`' order): Dış, Delik 1, Parça 2, Parça 2, delik 1. */
export function ringNames(e: Editable): string[] {
  // A multi-part polyline's parts (docs/adr/0174).
  if (e.kind === 'polyline' && e.parts) return Array.from({ length: e.parts.length + 1 }, (_, k) => `Parça ${k + 1}`);
  if (e.kind !== 'polygon') return [''];
  const out = ['Dış', ...(e.holes ?? []).map((_, i) => `Delik ${i + 1}`)];
  (e.parts ?? []).forEach((part, k) => {
    out.push(`Parça ${k + 2}`);
    (part.holes ?? []).forEach((_, i) => out.push(`Parça ${k + 2}, delik ${i + 1}`));
  });
  return out;
}

/** The row's key in the table's selection. */
const keyOf = (a: At) => `${a.path}:${a.index}`;

/** What a click on the row at `at` selects: it alone; with Ctrl the selection with it turned over; with Shift the run from `anchor`. */
export function clickRows(selected: ReadonlySet<string>, keys: readonly string[], at: number, anchor: number | null, how: { ctrl: boolean; shift: boolean }): string[] {
  const key = keys[at];
  if (how.shift && anchor !== null && anchor < keys.length) {
    const [a, b] = anchor <= at ? [anchor, at] : [at, anchor];
    return keys.slice(a, b + 1);
  }
  if (how.ctrl) return selected.has(key) ? [...selected].filter((k) => k !== key) : [...selected, key];
  return [key];
}

/**
 * Where the editor goes after a cell (docs/adr/0172 §4), by the rows' order: Enter down the column (to the next row
 * whose cell is edited), Tab right (Yarıçap to the next row's Y), Shift+Tab left; null past the ends.
 */
export function nextVertexCell(kind: VertexKind, rows: readonly VertexRow[], at: number, col: VertexColumn, how: 'down' | 'right' | 'left'): { at: number; col: VertexColumn } | null {
  if (how === 'down') {
    for (let r = at + 1; r < rows.length; r++) if (cellEditable(kind, rows[r], col)) return { at: r, col };
    return null;
  }
  const step = how === 'right' ? 1 : -1;
  let [r, c] = [at, VERTEX_COLUMNS.indexOf(col)];
  for (;;) {
    c += step;
    if (c < 0 || c >= VERTEX_COLUMNS.length) {
      r += step;
      if (r < 0 || r >= rows.length) return null;
      c = step > 0 ? 0 : VERTEX_COLUMNS.length - 1;
    }
    if (cellEditable(kind, rows[r], VERTEX_COLUMNS[c])) return { at: r, col: VERTEX_COLUMNS[c] };
  }
}

export class VertexTable extends Component {
  readonly el: HTMLElement;
  private readonly ctx: AppContext;
  private readonly id: number;
  /** Whether the table writes: one object selected, its layer not locked. */
  private readonly writes: boolean;
  private readonly note: string;
  private readonly scroller: HTMLElement;
  private readonly body = h('tbody');
  private readonly head = h('tr');
  private readonly foot = h('div', { class: 'ctable__foot num vtable__foot' });
  private readonly count = h('span', { class: 'ptable__count num' });
  private readonly showBtn: HTMLButtonElement;
  private readonly addBtn: HTMLButtonElement;
  private readonly removeBtn: HTMLButtonElement;
  private readonly rowsView: VirtualRows;
  private cols: Col[] = [];
  private rows: VertexRow[] = [];
  private paths: Elevated[] = [];
  private keys: string[] = [];
  private rings: string[] = [];
  /** The rows selected, by key; kept while the object keeps its vertices' count. */
  private selected = new Set<string>();
  private anchor: number | null = null;
  /** The vertex count the selection was made on: another count clears it. */
  private counted = -1;
  /** The cell edited: a vertex's, or the draft's (`at` null). */
  private editing: { at: At | null; col: VertexColumn } | null = null;
  /** What was typed in the cell edited, kept when its value was refused (the cell opens again with it). */
  private typed: string | null = null;
  private field: HTMLInputElement | null = null;
  /** Satır ekle's row: the vertex it goes after and its cells as typed. */
  private draft: { after: At; d: VertexDraft } | null = null;
  private pending = false;

  constructor(ctx: AppContext, id: number, how: { writes: boolean; note: string }) {
    super();
    this.ctx = ctx;
    this.id = id;
    this.writes = how.writes;
    this.note = how.note;
    const hint = h('span', { class: 'vtable__hint' }, how.writes ? VERTEX_TEXTS.hint : how.note === VERTEX_TEXTS.lockedNote ? VERTEX_TEXTS.locked : VERTEX_TEXTS.many);
    const button = (glyph: string, text: string, hint: string, run: () => void) => {
      const b = h('button', { class: 'btn btn--small ptable__btn', type: 'button' }, icon(glyph, 14), h('span', null, text));
      this.d.add(listen(b, 'click', run));
      this.d.add(tooltip(b, () => ({ title: text, description: hint }), 'top'));
      return b;
    };
    this.addBtn = button('plus', VERTEX_TEXTS.add, VERTEX_TEXTS.addHint, () => this.addRow());
    this.removeBtn = button('erase', VERTEX_TEXTS.remove, VERTEX_TEXTS.removeHint, () => this.removeSelected());
    this.showBtn = button('zoomSelection', VERTEX_TEXTS.show, VERTEX_TEXTS.showHint, () => this.show());
    // The table takes the keyboard when a row is pressed: Delete removes the selected rows' vertices there.
    this.scroller = h('div', { class: 'ptable__scroll', tabindex: '-1' }, h('table', null, h('thead', null, this.head), this.body));
    this.el = h(
      'div',
      { class: 'ptable vtable' },
      h(
        'div',
        { class: 'ptable__bar' },
        h('div', { class: 'ptable__group' }, hint),
        h('div', { class: 'ptable__group ptable__group--end' }, this.count, this.addBtn, this.removeBtn, this.showBtn),
      ),
      this.scroller,
      this.foot,
    );
    this.rowsView = new VirtualRows({ parent: this.body, scroller: this.scroller, row: (i) => this.row(i), spacer: tableSpacer(8) });
    this.d.add(() => this.rowsView.dispose());
    this.d.add(() => ctx.selection.vertices.set([]));

    this.d.add(
      listen<MouseEvent>(this.body, 'click', (e) => {
        const shown = this.rowAt(e);
        if (shown === null || (e.target as HTMLElement).closest('input')) return;
        const at = this.rowOfShown(shown);
        if (at === 'draft') return;
        this.scroller.focus({ preventScroll: true });
        const how = { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey };
        const next = clickRows(this.selected, this.keys, at, this.anchor, how);
        if (!how.shift) this.anchor = at;
        // A click on the selection as it is changes nothing: the rows stay, a double click's second press with them.
        if (next.length === this.selected.size && next.every((k) => this.selected.has(k))) return;
        this.selected = new Set(next);
        this.schedule();
      }),
    );
    this.d.add(
      listen<MouseEvent>(this.body, 'dblclick', (e) => {
        const shown = this.rowAt(e);
        const td = (e.target as HTMLElement).closest('td');
        if (shown === null || !td || td.querySelector('input')) return;
        const col = this.cols[td.cellIndex];
        const at = this.rowOfShown(shown);
        if (at === 'draft') {
          if (col?.edit && col.edit !== 'radius') this.edit({ at: null, col: col.edit });
          return;
        }
        if (col?.key === 'no') {
          this.selected = new Set([this.keys[at]]);
          this.anchor = at;
          this.show();
        } else if (col?.edit && this.writes && cellEditable(this.kind(), this.rows[at], col.edit)) this.edit({ at: { path: this.rows[at].path, index: this.rows[at].index }, col: col.edit });
      }),
    );
    this.d.add(
      listen<KeyboardEvent>(this.scroller, 'keydown', (e) => {
        if (e.key !== 'Delete' || (e.target as HTMLElement).closest('input')) return;
        // The table's, not the drawing's Sil: the object stays.
        e.preventDefault();
        this.removeSelected();
      }),
    );
    this.d.add(watchAll([ctx.format.changed], () => this.schedule()));
    this.d.add(ctx.doc.events.on('changed', () => this.schedule()));
    queueMicrotask(() => this.refresh());
  }

  /** Whether the object shown is still there as line work: the table follows it; else the list is built again. */
  get alive(): boolean {
    return this.entity() !== null;
  }

  /** The object shown, while it is line work. */
  private entity(): Editable | null {
    const e: Entity | undefined = this.ctx.doc.get(this.id);
    return isEditable(e) ? e : null;
  }

  private kind(): VertexKind {
    return (this.entity()?.kind ?? 'polyline') as VertexKind;
  }

  private rowAt(e: MouseEvent): number | null {
    const tr = (e.target as HTMLElement).closest<HTMLElement>('tr[data-at]');
    return tr ? Number(tr.dataset.at) : null;
  }

  /** Where the draft row stands among the rows shown (under its vertex's row), or -1. */
  private draftAt(): number {
    if (!this.draft) return -1;
    const i = this.keys.indexOf(keyOf(this.draft.after));
    return i < 0 ? -1 : i + 1;
  }

  /** What the row shown at `i` is: a vertex's row (its index in the rows) or the draft. */
  private rowOfShown(i: number): number | 'draft' {
    const d = this.draftAt();
    if (d < 0 || i < d) return i;
    return i === d ? 'draft' : i - 1;
  }

  private schedule(): void {
    if (this.pending) return;
    this.pending = true;
    queueMicrotask(() => {
      this.pending = false;
      this.refresh();
    });
  }

  /** The selected rows' vertices, in the table's order. */
  private marked(): VertexRow[] {
    return this.rows.filter((r) => this.selected.has(keyOf(r)));
  }

  /** Göster: the first selected vertex (the first row when none is) to the view's middle, its scale kept. */
  private show(): void {
    const row = this.marked()[0] ?? this.rows[0];
    if (row) this.ctx.view.centerOn(row.p);
    this.refresh();
  }

  /** Satır ekle: a draft row under the last selected row (the last row when none is), its Y open. */
  private addRow(): void {
    if (!this.writes) return;
    const after = this.marked().at(-1) ?? this.rows.at(-1);
    if (!after) return;
    this.draft = { after: { path: after.path, index: after.index }, d: emptyVertexDraft() };
    this.edit({ at: null, col: 'east' });
  }

  /** Sil: the selected rows' vertices removed in one step. */
  private removeSelected(): void {
    const e = this.entity();
    const at = this.marked().map((r) => ({ path: r.path, index: r.index }));
    if (!this.writes || !e || !at.length) return;
    this.say(removeVertices(this.ctx.doc, e, at, (v) => this.ctx.format.length(v), this.neighbours));
    this.refresh();
  }

  private columns(e: Editable): Col[] {
    const f = this.ctx.format;
    const unit = f.lengthUnitLabel;
    const cols: Col[] = [{ key: 'no', label: 'Köşe', numeric: true, edit: null }];
    if (this.rings.length > 1) cols.push({ key: 'ring', label: 'Halka', numeric: false, edit: null });
    cols.push(
      { key: 'east', label: f.axesText('Y (sağa)'), numeric: true, edit: 'east' },
      { key: 'north', label: f.axesText('X (yukarı)'), numeric: true, edit: 'north' },
      { key: 'z', label: 'Z (kot)', numeric: true, edit: 'z' },
    );
    // A line's edge takes no arc: its column would stay empty.
    if (e.kind !== 'line') cols.push({ key: 'radius', label: `Yarıçap (${unit})`, numeric: true, edit: 'radius' });
    cols.push({ key: 'chord', label: `Kenar (${unit})`, numeric: true, edit: null }, { key: 'bearing', label: `${f.directionName} (${f.angleUnitLabel})`, numeric: true, edit: null });
    return cols;
  }

  private refresh(): void {
    if (!this.el.isConnected) return;
    const e = this.entity();
    if (!e) return;
    this.paths = elevatedPaths(e);
    this.rows = vertexRows(e);
    this.keys = this.rows.map(keyOf);
    this.rings = ringNames(e);
    this.cols = this.columns(e);
    // Another vertex count (a vertex added or removed, an undo) clears the selection; a write in place keeps it.
    if (this.rows.length !== this.counted) {
      if (this.counted >= 0) this.selected.clear();
      this.counted = this.rows.length;
      this.anchor = null;
    }
    for (const k of [...this.selected]) if (!this.keys.includes(k)) this.selected.delete(k);
    // A draft whose vertex went away (an undo) goes with it.
    if (this.draft && this.draftAt() < 0) {
      this.draft = null;
      if (this.editing?.at === null) this.editing = null;
    }
    replaceChildren(
      this.head,
      this.cols.map((c) => h('th', { class: c.numeric ? 'num' : null, scope: 'col' }, h('span', { class: 'vtable__head' }, c.label))),
    );
    this.count.textContent = `${this.rows.length} köşe`;
    this.showBtn.disabled = this.rows.length === 0;
    this.addBtn.disabled = !this.writes || this.rows.length === 0;
    this.removeBtn.disabled = !this.writes || this.selected.size === 0;
    this.foot.textContent = this.footer(e);
    this.ctx.selection.vertices.set(this.marked().map((r) => r.p));
    this.field = null;
    const draft = this.draftAt();
    this.rowsView.set(this.rows.length + (draft >= 0 ? 1 : 0));
    if (this.editing) {
      const at = this.editing.at === null ? draft : this.keys.indexOf(keyOf(this.editing.at));
      if (at < 0) this.editing = null;
      else this.rowsView.reveal(this.editing.at !== null && draft >= 0 && at >= draft ? at + 1 : at);
    }
  }

  /** The object's name and its own measures, as the coordinate list says them; the note when the table does not write. */
  private footer(e: Editable): string {
    const f = this.ctx.format;
    const title = e.label ? `${e.attrs.Ada ? `${e.attrs.Ada} ada ` : ''}${e.label}` : `#${e.id}`;
    // The object's own measures (arcs followed, holes counted), as the properties panel shows them.
    const area = entityArea(e);
    const length = entityLength(e);
    const text =
      area !== null
        ? `${title}   Alan ${f.area(area)}${length !== null ? `   Çevre ${f.length(length)}` : ''}`
        : length !== null
          ? `${title}   Uzunluk ${f.length(length)}`
          : title;
    return this.note ? `${text}   ${this.note}` : text;
  }

  private row(shown: number): HTMLElement {
    const which = this.rowOfShown(shown);
    if (which === 'draft') return this.draftRow(shown);
    const i = which;
    const r = this.rows[i];
    const f = this.ctx.format;
    const path = this.paths[r.path];
    const n = path ? (r.index + 1 < path.pts.length ? r.index + 1 : path.closed ? 0 : null) : null;
    const next = n === null ? null : path.pts[n];
    const value = (c: Col): string => {
      switch (c.key) {
        case 'no':
          return String(i + 1);
        case 'ring':
          return this.rings[r.path] ?? '';
        case 'east':
          return f.coord(r.p.x);
        case 'north':
          return f.coord(r.p.y);
        case 'z':
          return r.z != null ? f.length(r.z, false) : '';
        case 'radius':
          return r.radius != null ? f.length(r.radius, false) : '';
        case 'chord':
          return r.chord != null ? f.length(r.chord, false) : '';
        case 'bearing':
          return next ? f.direction(bearingGrad(r.p, next), false) : '';
      }
    };
    const on = this.selected.has(keyOf(r));
    const at = this.editing?.at;
    const ed = at && at.path === r.path && at.index === r.index ? this.editing!.col : null;
    const perMetre = UNIT_PER_METRE[this.ctx.doc.settings.unit];
    return h(
      'tr',
      { class: on ? 'is-selected' : null, 'data-at': String(shown), 'aria-selected': String(on) },
      this.cols.map((c) => {
        const editing = c.edit !== null && c.edit === ed;
        const cls = `${c.numeric ? 'num' : ''}${editing ? ' is-editing' : ''}`.trim() || null;
        return h('td', { class: cls }, editing ? this.editor(this.typed ?? cellText(r, c.edit!, perMetre), c.numeric) : value(c));
      }),
    );
  }

  /** Satır ekle's row: Yeni, its ring, its cells as typed, the one edited open. */
  private draftRow(shown: number): HTMLElement {
    const d = this.draft?.d ?? emptyVertexDraft();
    const ed = this.editing?.at === null ? this.editing.col : null;
    const text: Record<string, string> = { east: d.east, north: d.north, z: d.z };
    return h(
      'tr',
      { class: 'ptable__draft', 'data-at': String(shown) },
      this.cols.map((c) => {
        const editing = c.edit !== null && c.edit === ed;
        const value = c.key === 'no' ? VERTEX_TEXTS.draft : c.key === 'ring' ? (this.rings[this.draft?.after.path ?? 0] ?? '') : (text[c.key] ?? '');
        const cls = `${c.numeric ? 'num' : ''}${editing ? ' is-editing' : ''}`.trim() || null;
        return h('td', { class: cls }, editing ? this.editor(value, c.numeric) : value);
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

  private edit(to: { at: At | null; col: VertexColumn } | null, typed: string | null = null): void {
    this.editing = to;
    this.typed = typed;
    this.refresh();
  }

  private cancel(): void {
    this.field = null;
    if (this.editing?.at === null) this.draft = null;
    this.edit(null);
    this.ctx.view.focus();
  }

  private say(out: Outcome): void {
    for (const line of out.told ?? []) this.ctx.log.info(line);
    for (const line of out.said) this.ctx.log.warn(line);
  }

  /** Topoloji (docs/adr/0172 §6): the neighbours a write takes along, while the mode is on. */
  private readonly neighbours = (before: Entity, after: Entity) => neighboursOf(this.ctx, [[before, after]]);

  /** The edit ends with `value`: written, then the editor goes `how` (Enter down, Tab right, Shift+Tab left; null stops). */
  private finish(value: string, how: 'down' | 'right' | 'left' | null): void {
    const editing = this.editing;
    this.field = null;
    const e = this.entity();
    if (!editing || !e) return this.edit(null);
    if (editing.at === null) return this.finishDraft(e, editing.col, value, how);
    const kind = e.kind as VertexKind;
    // Where to go next, by the rows before the write (a vertex's place does not change its row).
    const at = this.keys.indexOf(keyOf(editing.at));
    const next = how && at >= 0 ? nextVertexCell(kind, this.rows, at, editing.col, how) : null;
    const out = writeVertexCell(this.ctx.doc, e, editing.at, editing.col, value, (v) => this.ctx.format.length(v), this.neighbours);
    this.say(out);
    // A value refused: the cell stays open with what was typed (a click elsewhere gives it up).
    if (out.stay) return this.edit(how ? editing : null, how ? value : null);
    const row = next ? this.rows[next.at] : null;
    this.edit(next && row ? { at: { path: row.path, index: row.index }, col: next.col } : null);
    if (!next) this.ctx.view.focus();
  }

  /**
   * The draft's cell ends with `value`: Enter writes the vertex (the next draft opens under it, its Y open; a value
   * refused keeps the cell open), Tab and Shift+Tab walk its cells, a click elsewhere keeps what was typed.
   */
  private finishDraft(e: Editable, col: VertexColumn, value: string, how: 'down' | 'right' | 'left' | null): void {
    if (!this.draft) return this.edit(null);
    this.draft.d = { ...this.draft.d, [col]: value };
    if (how === null) return this.edit(null);
    if (how !== 'down') {
      const cells: VertexColumn[] = ['east', 'north', 'z'];
      const c = cells.indexOf(col) + (how === 'right' ? 1 : -1);
      return this.edit({ at: null, col: cells[(c + cells.length) % cells.length] });
    }
    const out = writeVertexDraft(this.ctx.doc, e, this.draft.after, this.draft.d, (v) => this.ctx.format.length(v), this.neighbours);
    this.say(out);
    if (!out.next) return this.edit({ at: null, col });
    this.draft = { after: out.next, d: emptyVertexDraft() };
    this.edit({ at: null, col: 'east' });
  }
}
