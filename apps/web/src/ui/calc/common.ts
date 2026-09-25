import '../../styles/calc.css';
import type { AppContext } from '../../app/context';
import type { Vec2 } from '../../model/geometry';
import type { NewEntity } from '../../model/entities';
import { PickPointTool } from '../../tools/pickPointTool';
import { h, replaceChildren, type Child } from '../dom';
import { icon } from '../icons';
import { field, select, summaryLine } from '../io/common';

/**
 * Parts the Hesap windows share (poligon, kutupsal alım, aplikasyon,
 * kestirmeler): a known point given by its name in the drawing or as
 * "Y,X" (or shown on the drawing), an editable table of measurements that
 * takes a paste from a spreadsheet, the results table, adding the new points
 * to the drawing as one undo step and copying the report. The computations
 * are the geometry core's (model/geom/surveyCalc.ts); what is typed stays
 * for the session, so a window closed to look at the drawing opens again
 * as it was left.
 */

/** A known point as typed: a point's name in the drawing, or "Y,X". */
export type Known = { p: Vec2; name: string } | { error: string } | null;

const COORDS = /^\s*(-?\d+(?:\.\d+)?)\s*[,;\s]\s*(-?\d+(?:\.\d+)?)\s*$/;

/** Reads a known point: "Y,X" (Y east first, as on the command line) or the name of a point object. */
export function resolvePoint(ctx: AppContext, text: string): Known {
  const t = text.trim();
  if (!t) return null;
  const m = COORDS.exec(t);
  if (m) return { p: { x: Number(m[1]), y: Number(m[2]) }, name: '' };
  const key = t.toLocaleUpperCase('tr-TR');
  const found = [...ctx.doc.all()].filter((e) => e.kind === 'point' && (e.label ?? e.attrs.Ad ?? '').toLocaleUpperCase('tr-TR') === key);
  if (!found.length) return { error: `“${t}” adlı nokta çizimde yok. Adını denetleyin ya da Y,X yazın.` };
  const e = found[0];
  if (e.kind !== 'point') return null;
  return { p: e.p, name: e.label ?? t };
}

/** The name a point object at exactly `p` has, if any (a picked point snapped to a named point). */
function nameAt(ctx: AppContext, p: Vec2): string | null {
  for (const e of ctx.doc.all()) if (e.kind === 'point' && e.p.x === p.x && e.p.y === p.y && e.label) return e.label;
  return null;
}

/** A number as typed: a decimal comma is taken for a point. Empty: null; not a number: NaN. */
export function readNumber(text: string | undefined): number | null {
  const t = (text ?? '').trim().replace(',', '.');
  if (!t) return null;
  return /^[-+]?(\d+(\.\d*)?|\.\d+)(e[-+]?\d+)?$/i.test(t) ? Number(t) : Number.NaN;
}

/** What a window needs to let the user show a point on the drawing and come back. */
export interface Picker {
  ctx: AppContext;
  title: string;
  /** Closes the window (its state stays) and opens it again after the pick. */
  close(): void;
  reopen(): void;
}

/**
 * A known point field: the text (a name or Y,X), a button to show the point
 * on the drawing, and under it what it resolves to.
 */
export function knownField(pick: Picker, label: string, state: { text: string }, onChange: () => void, key: string, hint?: string): HTMLElement {
  const { ctx } = pick;
  const input = h('input', { class: 'field calc-known__input', type: 'text', value: state.text, placeholder: 'Nokta adı ya da Y,X', 'aria-label': label, dataset: { key } });
  const resolved = h('span', { class: 'calc-known__value' });
  const show = () => {
    const r = resolvePoint(ctx, state.text);
    resolved.dataset.kind = r && 'error' in r ? 'error' : '';
    resolved.textContent = !r ? (hint ?? 'Henüz verilmedi') : 'error' in r ? r.error : ctx.format.point(r.p);
  };
  input.addEventListener('input', () => {
    state.text = input.value;
    show();
    onChange();
  });
  const button = h('button', { class: 'btn calc-known__pick', type: 'button', title: 'Çizimde gösterin (bir noktaya kenetlenirse adı alınır)' }, icon('snap', 14), 'Çizimden');
  button.addEventListener('click', () => {
    pick.close();
    ctx.tools.run(
      new PickPointTool(ctx, `${pick.title}: ${label}`, (p) => {
        if (p) state.text = nameAt(ctx, p) ?? `${p.x},${p.y}`;
        queueMicrotask(() => pick.reopen());
      }),
      `${pick.title}: ${label}`,
    );
  });
  show();
  return field(label, h('div', { class: 'calc-known' }, h('div', { class: 'calc-known__row' }, input, button), resolved));
}

export interface GridColumn {
  key: string;
  label: string;
  /** Shown after the label ("g", "m"). */
  unit?: string;
  numeric?: boolean;
  placeholder?: (row: number) => string;
}

export type Row = Record<string, string>;

/** The rows a table shows and what may be done to them. */
export interface GridModel {
  columns: GridColumn[];
  rows(): Row[];
  /** A cell that cannot be typed in (a known station's name, a leg after the last point). */
  readonly(row: number, key: string): boolean;
  /** Whether a new row may follow this one, and adds it. */
  canInsertAfter(row: number): boolean;
  insertAfter(row: number): void;
  canRemove(row: number): boolean;
  remove(row: number): void;
}

/**
 * An editable measurements table. Enter or ↓ goes down a column and adds a
 * row at the end; a paste of several lines or columns (from a spreadsheet or
 * field book export, tab, semicolon or space separated) fills from the cell
 * down and right, adding rows as needed.
 */
export class Grid {
  readonly el = h('div', { class: 'calc-grid' });
  private readonly model: GridModel;
  private readonly onChange: () => void;

  constructor(model: GridModel, onChange: () => void) {
    this.model = model;
    this.onChange = onChange;
    this.render();
  }

  render(focus?: { row: number; key: string }): void {
    const { model } = this;
    const rows = model.rows();
    const head = h(
      'tr',
      null,
      h('th', { class: 'calc-grid__no' }, '#'),
      model.columns.map((c) => h('th', null, c.label, c.unit ? h('span', { class: 'calc-grid__unit' }, ` (${c.unit})`) : null)),
      h('th', { class: 'calc-grid__act' }),
    );
    const body = rows.map((row, r) =>
      h(
        'tr',
        null,
        h('td', { class: 'calc-grid__no' }, String(r + 1)),
        model.columns.map((c) => {
          if (model.readonly(r, c.key)) return h('td', { class: 'calc-grid__fixed' }, row[c.key] || '—');
          const input = h('input', {
            class: `calc-grid__cell${c.numeric ? ' num' : ''}`,
            type: 'text',
            value: row[c.key] ?? '',
            placeholder: c.placeholder?.(r) ?? '',
            'aria-label': `${r + 1}. satır ${c.label}`,
            dataset: { row: String(r), key: c.key },
          });
          input.addEventListener('input', () => {
            row[c.key] = input.value;
            input.toggleAttribute('data-bad', !!c.numeric && Number.isNaN(readNumber(input.value) ?? 0));
            this.onChange();
          });
          input.addEventListener('keydown', (e) => this.key(e, r, c.key));
          input.addEventListener('paste', (e) => this.paste(e, r, c.key));
          return h('td', null, input);
        }),
        h(
          'td',
          { class: 'calc-grid__act' },
          model.canRemove(r)
            ? (() => {
                const b = h('button', { class: 'ibtn', type: 'button', title: 'Satırı sil', 'aria-label': `${r + 1}. satırı sil` }, icon('close', 12));
                b.addEventListener('click', () => {
                  model.remove(r);
                  this.render();
                  this.onChange();
                });
                return b;
              })()
            : null,
        ),
      ),
    );
    const add = h('button', { class: 'btn calc-grid__add', type: 'button' }, icon('plus', 14), 'Satır ekle');
    const last = rows.length - 1;
    const after = [...rows.keys()].reverse().find((r) => model.canInsertAfter(r)) ?? -1;
    add.disabled = after < 0 && last >= 0;
    add.addEventListener('click', () => {
      model.insertAfter(after);
      this.render({ row: after + 1, key: model.columns.find((c) => !model.readonly(after + 1, c.key))?.key ?? model.columns[0].key });
      this.onChange();
    });
    replaceChildren(this.el, h('div', { class: 'calc-grid__wrap' }, h('table', { class: 'io-table calc-grid__table' }, h('thead', null, head), h('tbody', null, body))), add);
    if (focus) this.focus(focus.row, focus.key);
  }

  private focus(row: number, key: string): void {
    this.el.querySelector<HTMLInputElement>(`input[data-row="${row}"][data-key="${key}"]`)?.focus();
  }

  private key(e: KeyboardEvent, r: number, key: string): void {
    const { model } = this;
    const down = e.key === 'Enter' || e.key === 'ArrowDown';
    if (!down && e.key !== 'ArrowUp') return;
    e.preventDefault();
    if (e.key === 'ArrowUp') return this.focus(Math.max(0, r - 1), key);
    const rows = model.rows();
    // The next row with this column open, or a new one.
    for (let n = r + 1; n < rows.length; n++) if (!model.readonly(n, key)) return this.focus(n, key);
    if (!model.canInsertAfter(r)) return;
    model.insertAfter(r);
    this.render({ row: r + 1, key });
    this.onChange();
  }

  private paste(e: ClipboardEvent, r: number, key: string): void {
    const text = e.clipboardData?.getData('text/plain') ?? '';
    if (!/[\t;\n]/.test(text.trim())) return;
    e.preventDefault();
    const { model } = this;
    const lines = text.replace(/\r/g, '').split('\n').filter((l) => l.trim());
    const cols = model.columns.map((c) => c.key);
    const start = cols.indexOf(key);
    let row = r;
    for (const [i, line] of lines.entries()) {
      if (i > 0) {
        if (row + 1 >= model.rows().length || model.readonly(row + 1, key)) {
          if (!model.canInsertAfter(row)) break;
          model.insertAfter(row);
        }
        row += 1;
      }
      const cells = line.split(/\t|;|\s{2,}|\s(?=[-+\d.])/).map((c) => c.trim());
      const target = model.rows()[row];
      cells.forEach((v, j) => {
        const k = cols[start + j];
        if (k && !model.readonly(row, k)) target[k] = v;
      });
    }
    this.render({ row, key });
    this.onChange();
  }
}

/** A results table: header cells and rows of text (numbers aligned right). */
export function resultTable(head: string[], rows: Child[][], numeric: readonly boolean[]): HTMLElement {
  return h(
    'div',
    { class: 'io-table-wrap calc-results' },
    h(
      'table',
      { class: 'io-table' },
      h('thead', null, h('tr', null, head.map((c, i) => h('th', { class: numeric[i] ? 'num' : '' }, c)))),
      h('tbody', null, rows.map((r) => h('tr', null, r.map((c, i) => h('td', { class: numeric[i] ? 'num' : '' }, c))))),
    ),
  );
}

/** An angle already in the project's unit, with its unit mark. */
export function angleText(ctx: AppContext, v: number): string {
  return `${v.toFixed(4)}${ctx.format.angleUnitLabel === '°' ? '°' : ' g'}`;
}

/** A small angle (a misclosure) with its fine unit: grad with cc (10⁻⁴ g), degrees with seconds. */
export function smallAngleText(ctx: AppContext, v: number): string {
  if (ctx.format.angleUnitLabel === '°') return `${unsigned(v.toFixed(5))}° (${unsigned((v * 3600).toFixed(1))}″)`;
  return `${unsigned(v.toFixed(5))} g (${unsigned((v * 10000).toFixed(1))} cc)`;
}

/** A rounded value that came out as −0 reads as 0. */
const unsigned = (text: string): string => (/^-0(\.0*)?$/.test(text) ? text.slice(1) : text);

/** A small length (a misclosure) in millimetres; a rounded −0 reads as 0. */
export function mmText(m: number): string {
  return `${unsigned((m * 1000).toFixed(1))} mm`;
}

/** A new point for the drawing. */
export interface NewPoint {
  name: string;
  p: Vec2;
  z?: number | null;
}

/** Where the new points go: a layer picker (defaults to `preferred` when it exists, else the active layer). */
export function layerChoice(ctx: AppContext, state: { layer: string | null }, preferred: string): HTMLElement {
  const layers = ctx.doc.layers;
  const leaves = layers.leaves();
  if (!state.layer || !layers.get(state.layer)) state.layer = layers.get(preferred) ? preferred : layers.active.value;
  const s = select(
    'Katman',
    leaves.map((l) => ({ value: l.id, label: `${l.name}${layers.isLocked(l.id) ? ' (kilitli)' : ''}`, disabled: layers.isLocked(l.id) })),
    state.layer,
    (v) => (state.layer = v),
    'layer',
  );
  s.classList.add('calc-layer');
  return s;
}

/**
 * Adds the points to the layer as one undo step, with their names as labels
 * and attributes (Ad, Tür, Z). Returns how many were added, or null with a
 * message when the layer cannot take them.
 */
export function addPoints(ctx: AppContext, layerId: string, points: readonly NewPoint[], kind: string, label: string): number | null {
  const layers = ctx.doc.layers;
  const node = layers.get(layerId);
  if (!node) return null;
  if (layers.isLocked(layerId)) {
    ctx.log.warn(`“${node.name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katman seçin.`);
    return null;
  }
  const entities: NewEntity[] = points.map((pt) => ({
    kind: 'point',
    layerId,
    p: pt.p,
    ...(pt.z != null ? { z: pt.z } : {}),
    label: pt.name,
    attrs: { Ad: pt.name, Tür: kind, ...(pt.z != null ? { 'Z (m)': pt.z.toFixed(3) } : {}) },
  }));
  const added = ctx.doc.transact(label, () => ctx.doc.addMany(entities, label));
  ctx.selection.set(added.map((e) => e.id));
  if (!layers.isVisible(layerId)) ctx.log.warn(`“${node.name}” katmanı gizli; eklenen noktalar görünmüyor.`);
  return added.length;
}

/** Copies a report (tab-separated lines, pastes into a spreadsheet) and says so. */
export function copyReport(ctx: AppContext, title: string, lines: string[][]): void {
  const text = lines.map((l) => l.join('\t')).join('\n');
  void navigator.clipboard.writeText(text).then(
    () => ctx.log.success(`${title} raporu panoya kopyalandı (${lines.length} satır; elektronik tabloya yapıştırılabilir).`),
    () => ctx.log.warn('Rapor panoya kopyalanamadı: tarayıcı izin vermedi.'),
  );
}

/** The summary box's lines. */
export function summary(el: HTMLElement, lines: (HTMLElement | null)[]): void {
  replaceChildren(el, lines.filter((l): l is HTMLElement => !!l));
  el.hidden = !el.childElementCount;
}

export { field, summaryLine };
