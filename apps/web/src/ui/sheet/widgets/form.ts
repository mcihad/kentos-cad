import type { DisposableStore } from '../../../core/disposable';
import { h, type Child } from '../../dom';
import { Dropdown } from '../../widgets/Dropdown';
import type { MenuItem } from '../../widgets/PopupMenu';
import { tooltip } from '../../widgets/tooltip';

/**
 * The sheet windows' form parts (Sayfa ayarları, Değişkenler, Şablon olarak
 * kaydet, Dışa aktar, the inspector's kind sections): a label over its
 * field, a choice from a list, a line of text, a long text, in the style
 * windows' layout (DESIGN.md §7.14). A window draws itself again when a
 * value changes; these parts keep nothing of their own.
 */

/** A labelled field: the label over it, a hint under it. */
export function field(label: string, control: Child, hint?: string | null, wide = true): HTMLElement {
  return h('div', { class: `sheet-field${wide ? ' sheet-field--wide' : ''}` }, h('span', { class: 'sheet-field__label' }, label), control, hint ? h('p', { class: 'sheet-insp__hint' }, hint) : null);
}

export interface ChoiceOption<T> {
  readonly value: T;
  readonly label: string;
  readonly detail?: string;
  readonly disabled?: boolean;
}

/** A choice from a list (a drop-down whose rows are radio items); “—” when the chosen items differ (`value` null). */
export function choice<T>(o: { label: string; options: readonly ChoiceOption<T>[]; value: T | null; readOnly?: string | null; onChange(v: T): void; key?: string }, d: DisposableStore): HTMLElement {
  const dd = new Dropdown({
    ariaLabel: o.label,
    className: 'dropdown--cell',
    items: (): MenuItem[] => o.options.map((x) => ({ label: x.label, detail: x.detail, radio: true, checked: x.value === o.value, disabled: x.disabled, run: () => o.onChange(x.value) })),
  });
  const shown = o.options.find((x) => x.value === o.value);
  dd.set(h('span', { class: 'dropdown__text' }, o.value === null ? '—' : (shown?.label ?? String(o.value))));
  if (o.key) dd.el.dataset.key = o.key;
  dd.el.disabled = !!o.readOnly;
  const wrap = h('div', { class: 'sheet-choice' }, dd.el);
  // A list that cannot be changed now does not open (the list's own press would open it, disabled or not).
  wrap.addEventListener(
    'pointerdown',
    (e) => {
      if (!dd.el.disabled) return;
      e.preventDefault();
      e.stopPropagation();
    },
    true,
  );
  if (o.readOnly) d.add(tooltip(wrap, () => ({ title: o.label, note: o.readOnly ?? undefined })));
  return field(o.label, wrap);
}

/** One line of text, taken when it loses the focus or with Enter (Esc puts it back). */
export function textInput(o: { label: string; value: string | null; readOnly?: string | null; placeholder?: string; key?: string; onCommit(v: string): void }, d: DisposableStore): HTMLElement {
  const shown = o.value ?? '';
  const input = h('input', {
    class: 'sheet-field__input',
    value: shown,
    placeholder: o.value === null ? '—' : (o.placeholder ?? null),
    spellcheck: 'false',
    'aria-label': o.label,
    readonly: !!o.readOnly,
    dataset: { key: o.key ?? o.label, escape: 'local' },
  });
  const commit = () => {
    if (input.readOnly || input.value === shown) return;
    o.onCommit(input.value);
  };
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      commit();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      input.value = shown;
      input.blur();
    }
  });
  input.addEventListener('blur', commit);
  if (o.readOnly) d.add(tooltip(input, () => ({ title: o.label, note: o.readOnly ?? undefined })));
  return field(o.label, h('div', { class: 'sheet-field__box' }, input));
}

/** A text of several lines (a text item's content, a cell's value): taken when it loses the focus or with Ctrl+Enter. */
export function longText(o: { label: string; value: string | null; readOnly?: string | null; rows?: number; key?: string; onCommit(v: string): void; extra?: Child }, d: DisposableStore): HTMLElement {
  const shown = o.value ?? '';
  const area = h('textarea', {
    class: 'sheet-field__area',
    rows: String(o.rows ?? 3),
    spellcheck: 'false',
    placeholder: o.value === null ? '—' : null,
    'aria-label': o.label,
    readonly: !!o.readOnly,
    dataset: { key: o.key ?? o.label, escape: 'local' },
  }) as HTMLTextAreaElement;
  area.value = shown;
  const commit = () => {
    if (area.readOnly || area.value === shown) return;
    o.onCommit(area.value);
  };
  area.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      commit();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      area.value = shown;
      area.blur();
    }
  });
  area.addEventListener('blur', commit);
  if (o.readOnly) d.add(tooltip(area, () => ({ title: o.label, note: o.readOnly ?? undefined })));
  return field(o.label, h('div', { class: 'sheet-field__box sheet-field__box--area' }, area, o.extra ?? null));
}

/** A colour (`#rrggbb`), the browser's own picker; “—” beside it when the chosen items differ. */
export function colorInput(o: { label: string; value: string | null; readOnly?: string | null; key?: string; onCommit(v: string): void }, d: DisposableStore): HTMLElement {
  const input = h('input', { class: 'sheet-color', type: 'color', value: (o.value ?? '#000000').slice(0, 7), 'aria-label': o.label, disabled: !!o.readOnly, dataset: { key: o.key ?? o.label } });
  input.addEventListener('change', () => o.onCommit(input.value));
  if (o.readOnly) d.add(tooltip(input, () => ({ title: o.label, note: o.readOnly ?? undefined })));
  return field(o.label, h('div', { class: 'sheet-color__row' }, input, h('span', { class: 'sheet-color__text num' }, o.value === null ? '—' : o.value)), null, false);
}
