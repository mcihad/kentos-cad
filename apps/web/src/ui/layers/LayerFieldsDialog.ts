import type { AppContext } from '../../app/context';
import { DisposableStore, listen } from '../../core/disposable';
import { Refusal } from '../../model/document';
import { checkValue, FIELD_KINDS, fieldsProblem, inferFields, type FieldChoice, type LayerField, type LayerFieldKind } from '../../model/layerFields';
import { h, replaceChildren } from '../dom';
import { field as labelled, summaryLine } from '../io/common';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';
import { askUnsaved } from '../widgets/confirm';

/** The window's title, which a trace names it by. */
export const FIELDS_TITLE = 'Alanlar';

/**
 * Alanlar (docs/adr/0199 §3; the desktop's `layer_fields.rs`): a layer's fields in a table, each its name, alias,
 * kind, length or decimals, whether it is required, its default, its range and its value list (Liste… opens the codes
 * and labels). Alan ekle, Sil, Yukarı, Aşağı; Verilerden al adds a field for each key the layer's objects carry that
 * the table has not, of the first kind all its values take. The list's first problem is said under the table and
 * Kaydet waits for it. Kaydet writes the fields and the renamed fields' keys on the objects as one undo step
 * (“Alanlar”); the values that do not keep the new rules are counted.
 */

/** A field as the window edits it: every value as typed, and the name it had when the window opened (none: new). */
export interface FieldRow {
  from: string | null;
  name: string;
  alias: string;
  kind: LayerFieldKind;
  /** A text's length or a decimal's fraction digits, as typed. */
  size: string;
  required: boolean;
  default: string;
  min: string;
  max: string;
  values: FieldChoice[] | null;
}

/** A field as the window shows it. */
export function rowOf(f: LayerField): FieldRow {
  return {
    from: f.name,
    name: f.name,
    alias: f.alias ?? '',
    kind: f.kind,
    size: String(f.kind === 'text' ? (f.length ?? '') : f.kind === 'decimal' ? (f.scale ?? '') : ''),
    required: f.required === true,
    default: f.default ?? '',
    min: f.min ?? '',
    max: f.max ?? '',
    values: f.values ? f.values.map((c) => ({ ...c })) : null,
  };
}

/** A whole number as typed, or the text itself when it is none (the rules then refuse it with the field's name). */
const count = (text: string): number | undefined => (text.trim() === '' ? undefined : /^\d+$/.test(text.trim()) ? Number(text.trim()) : Number.NaN);

/** The field a row writes: what is typed, a kind's own parts only (a size, a range, a list where the kind takes one). */
export function fieldOf(r: FieldRow): LayerField {
  const f: LayerField = { name: r.name, kind: r.kind };
  if (r.alias.trim()) f.alias = r.alias;
  const size = count(r.size);
  if (size !== undefined && r.kind === 'text') f.length = size;
  if (size !== undefined && r.kind === 'decimal') f.scale = size;
  if (r.kind === 'integer' || r.kind === 'decimal') {
    if (r.min.trim()) f.min = r.min;
    if (r.max.trim()) f.max = r.max;
  }
  if (r.values && (r.kind === 'text' || r.kind === 'integer' || r.kind === 'decimal')) f.values = r.values.map((c) => ({ ...c }));
  if (r.required) f.required = true;
  if (r.default.trim()) f.default = r.default;
  return f;
}

/** Why a size typed is no number (the rules' own message names the range). */
function sizeProblem(rows: readonly FieldRow[]): string | null {
  for (const r of rows) if (Number.isNaN(count(r.size) ?? 0)) return `“${r.name}” alanının ${r.kind === 'text' ? 'uzunluğu' : 'ondalık basamağı'} bir tam sayı olmalı.`;
  return null;
}

/** The renamed fields' keys: (old, new) for each field kept under another name. */
export const renamesOf = (rows: readonly FieldRow[]): [string, string][] => rows.flatMap((r) => (r.from !== null && r.from !== r.name ? [[r.from, r.name] as [string, string]] : []));

/** Opens Alanlar for `layerId` (a layer, not a group). */
export function openLayerFields(ctx: AppContext, layerId: string): void {
  const { doc } = ctx;
  const node = doc.layers.get(layerId);
  if (!node || node.type !== 'layer') return;
  const d = new DisposableStore();
  const saved = (node.fields ?? []).map(rowOf);
  let rows: FieldRow[] = saved.map((r) => ({ ...r, values: r.values ? r.values.map((c) => ({ ...c })) : null }));
  let chosen = rows.length ? 0 : -1;

  const body = h('tbody');
  const summary = h('div', { class: 'io-summary fields-summary' });
  const button = (words: string, glyph: string | null, primary = false) =>
    h('button', { class: primary ? 'btn btn--primary' : 'btn', type: 'button' }, glyph ? icon(glyph, 14) : null, words) as HTMLButtonElement;
  const add = button('Alan ekle', 'plus');
  const remove = button('Sil', 'erase');
  const up = button('Yukarı', 'chevronUp');
  const down = button('Aşağı', 'chevronDown');
  const fromData = button('Verilerden al', 'fieldsFromData');
  const save = button('Kaydet', null, true);
  const cancel = button('Vazgeç', null);

  const changed = () => JSON.stringify(rows.map(fieldOf)) !== JSON.stringify(saved.map(fieldOf)) || renamesOf(rows).length > 0;
  const problem = () => sizeProblem(rows) ?? fieldsProblem(rows.map(fieldOf));

  /** One cell's input: typed into the row's field; the table's summary follows. */
  const input = (r: FieldRow, key: 'name' | 'alias' | 'size' | 'default' | 'min' | 'max', label: string, disabled = false, numeric = false) => {
    const el = h('input', { class: `field fields-cell${numeric ? ' num' : ''}`, type: 'text', value: r[key], 'aria-label': label, placeholder: key === 'alias' ? 'Takma ad' : null, spellcheck: 'false', disabled }) as HTMLInputElement;
    el.addEventListener('input', () => {
      r[key] = el.value;
      check();
    });
    return el;
  };

  function rowEl(r: FieldRow, i: number): HTMLElement {
    const kind = h(
      'select',
      { class: 'field fields-cell', 'aria-label': 'Tür' },
      FIELD_KINDS.map((k) => h('option', { value: k.kind, selected: k.kind === r.kind }, k.label)),
    ) as HTMLSelectElement;
    kind.addEventListener('change', () => {
      r.kind = kind.value as LayerFieldKind;
      render();
    });
    const sized = r.kind === 'text' || r.kind === 'decimal';
    const ranged = r.kind === 'integer' || r.kind === 'decimal';
    const listed = r.kind === 'text' || ranged;
    const must = h('input', { type: 'checkbox', checked: r.required, 'aria-label': 'Zorunlu' }) as HTMLInputElement;
    must.addEventListener('change', () => {
      r.required = must.checked;
      check();
    });
    const list = h('button', { class: 'btn btn--small', type: 'button', disabled: !listed }, r.values?.length ? `Liste (${r.values.length})` : 'Liste…') as HTMLButtonElement;
    list.addEventListener('click', () => openChoices(r, () => render()));
    const tr = h(
      'tr',
      { class: i === chosen ? 'is-selected' : null, 'aria-selected': String(i === chosen) },
      h('td', null, input(r, 'name', 'Ad')),
      h('td', null, input(r, 'alias', 'Takma ad')),
      h('td', null, kind),
      h('td', null, input(r, 'size', r.kind === 'decimal' ? 'Ondalık basamak' : 'Uzunluk', !sized, true)),
      h('td', { class: 'fields-check' }, must),
      h('td', null, input(r, 'default', 'Varsayılan')),
      h('td', null, input(r, 'min', 'En az', !ranged, true)),
      h('td', null, input(r, 'max', 'En çok', !ranged, true)),
      h('td', null, list),
    );
    tr.addEventListener('mousedown', () => {
      if (chosen !== i) {
        chosen = i;
        for (const [k, row] of [...body.children].entries()) row.classList.toggle('is-selected', k === i);
        buttons();
      }
    });
    return tr;
  }

  function buttons(): void {
    remove.disabled = chosen < 0;
    up.disabled = chosen <= 0;
    down.disabled = chosen < 0 || chosen >= rows.length - 1;
  }

  function check(): void {
    const p = problem();
    const kept = rows.filter((r) => r.from !== null && r.from !== r.name).length;
    replaceChildren(
      summary,
      ...(p
        ? [summaryLine('error', p)]
        : [summaryLine('ok', rows.length ? `${rows.length} alan${kept ? `; ${kept} alanın adı değişiyor, değerleri yeni adla taşınacak` : ''}.` : 'Alan yok: katmanın öznitelikleri serbest.')]),
    );
    save.disabled = p !== null || !changed();
  }

  function render(): void {
    replaceChildren(body, ...rows.map((r, i) => rowEl(r, i)));
    buttons();
    check();
  }

  /** The values the new fields refuse among the layer's objects, counted. */
  function refusedValues(fields: readonly LayerField[]): number {
    let n = 0;
    for (const e of doc.byLayer(layerId))
      for (const f of fields) {
        const v = e.attrs[f.name];
        if (v !== undefined && v.trim() && 'error' in checkValue(f, v)) n++;
      }
    return n;
  }

  function write(): boolean {
    const fields = rows.map(fieldOf);
    try {
      doc.setLayerFields(layerId, fields, renamesOf(rows));
    } catch (e) {
      if (!(e instanceof Refusal)) throw e;
      replaceChildren(summary, summaryLine('error', e.message));
      return false;
    }
    const bad = refusedValues(fields);
    ctx.log.info(`“${node!.name}” katmanının alanları kaydedildi.${bad ? ` ${bad} değer alanların kurallarına uymuyor; Tablo'da uyarı rengiyle gösterilir.` : ''}`);
    return true;
  }

  add.addEventListener('click', () => {
    rows.push({ from: null, name: `Alan ${rows.length + 1}`, alias: '', kind: 'text', size: '', required: false, default: '', min: '', max: '', values: null });
    chosen = rows.length - 1;
    render();
    (body.lastElementChild?.querySelector('input') as HTMLInputElement | null)?.select();
  });
  remove.addEventListener('click', () => {
    if (chosen < 0) return;
    rows.splice(chosen, 1);
    chosen = Math.min(chosen, rows.length - 1);
    render();
  });
  const move = (by: number) => {
    const to = chosen + by;
    if (chosen < 0 || to < 0 || to >= rows.length) return;
    [rows[chosen], rows[to]] = [rows[to], rows[chosen]];
    chosen = to;
    render();
  };
  up.addEventListener('click', () => move(-1));
  down.addEventListener('click', () => move(1));
  fromData.addEventListener('click', () => {
    const taken = new Set(rows.flatMap((r) => [r.name, r.from ?? '']));
    const found = inferFields(doc.byLayer(layerId).map((e) => e.attrs)).filter((f) => !taken.has(f.name));
    rows = [...rows, ...found.map((f) => ({ ...rowOf(f), from: f.name }))];
    if (chosen < 0 && rows.length) chosen = 0;
    render();
    if (!found.length) replaceChildren(summary, summaryLine('info', 'Nesnelerin her anahtarı tabloda: eklenecek alan yok.'));
  });

  const table = h(
    'table',
    { class: 'fields-table' },
    h(
      'thead',
      null,
      h('tr', null, ['Ad', 'Takma ad', 'Tür', 'Uzunluk / Basamak', 'Zorunlu', 'Varsayılan', 'En az', 'En çok', 'Değer listesi'].map((t) => h('th', { scope: 'col' }, t))),
    ),
    body,
  );
  // The columns' widths (io.css): the kind's list shows its longest name whole.
  table.prepend(h('colgroup', null, ['name', 'alias', 'kind', 'size', 'must', 'default', 'min', 'max', 'list'].map((c) => h('col', { class: `fields-col--${c}` }))));
  const dialog = new Dialog({
    title: FIELDS_TITLE,
    width: 980,
    className: 'dialog--io dialog--fields',
    content: [h('p', { class: 'fields-layer' }, `Katman: ${doc.layers.path(layerId)}`), h('div', { class: 'fields-scroll' }, table), h('div', { class: 'io-row fields-actions' }, add, remove, up, down, h('span', { class: 'fields-gap' }), fromData), summary],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, save],
    beforeClose: () => {
      if (!changed()) return true;
      void askUnsaved({ name: FIELDS_TITLE, after: 'Pencere kapanırsa bu değişiklikler kaybolur.', verb: 'kapat', canSave: problem() === null }).then((a) => {
        if (a === 'stay') return;
        if (a === 'save' && !write()) return;
        rows = saved;
        dialog.close();
      });
      return false;
    },
    onClose: () => d.dispose(),
  });
  d.add(listen(save, 'click', () => write() && ((rows = saved), dialog.close())));
  d.add(listen(cancel, 'click', () => ((rows = saved.map((r) => ({ ...r }))), dialog.close())));
  render();
}

/** Liste…: a field's value list, each a code and its label; Tamam writes it to the row (an empty list takes it off). */
function openChoices(r: FieldRow, done: () => void): void {
  let list: FieldChoice[] = (r.values ?? []).map((c) => ({ ...c }));
  const body = h('tbody');
  const render = () =>
    replaceChildren(
      body,
      ...list.map((c, i) => {
        const code = h('input', { class: 'field fields-cell', type: 'text', value: c.code, 'aria-label': 'Kod', spellcheck: 'false' }) as HTMLInputElement;
        const label = h('input', { class: 'field fields-cell', type: 'text', value: c.label, 'aria-label': 'Etiket', spellcheck: 'false' }) as HTMLInputElement;
        code.addEventListener('input', () => (c.code = code.value));
        label.addEventListener('input', () => (c.label = label.value));
        const drop = h('button', { class: 'ibtn', type: 'button', 'aria-label': 'Sil', title: 'Sil' }, icon('close', 14));
        drop.addEventListener('click', () => {
          list.splice(i, 1);
          render();
        });
        return h('tr', null, h('td', null, code), h('td', null, label), h('td', null, drop));
      }),
    );
  const add = h('button', { class: 'btn', type: 'button' }, icon('plus', 14), 'Değer ekle');
  add.addEventListener('click', () => {
    list = [...list, { code: '', label: '' }];
    render();
    (body.lastElementChild?.querySelector('input') as HTMLInputElement | null)?.focus();
  });
  const ok = h('button', { class: 'btn btn--primary', type: 'button' }, 'Tamam');
  const no = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const dialog = new Dialog({
    title: `Değer listesi · ${r.name}`,
    width: 460,
    stack: true,
    className: 'dialog--io dialog--choices',
    content: [
      labelled(
        'Kodlar ve etiketleri',
        h(
          'table',
          { class: 'fields-table' },
          h('colgroup', null, h('col', { class: 'fields-col--code' }), h('col', { class: 'fields-col--label' }), h('col', { class: 'fields-col--remove' })),
          h('thead', null, h('tr', null, h('th', null, 'Kod'), h('th', null, 'Etiket'), h('th', null, ''))),
          body,
        ),
        'Nesneye kod yazılır, tabloda ve formda etiketi gösterilir; etiket yazılırsa kodu yazılır.',
      ),
      h('div', { class: 'io-row' }, add),
    ],
    footer: [h('div', { class: 'dialog__spacer' }), no, ok],
  });
  ok.addEventListener('click', () => {
    r.values = list.length ? list : null;
    dialog.close();
    done();
  });
  no.addEventListener('click', () => dialog.close());
  render();
}
