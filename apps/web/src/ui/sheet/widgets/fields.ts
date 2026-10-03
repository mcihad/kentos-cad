import type { DisposableStore } from '../../../core/disposable';
import { fixed } from '../../../core/displayNumber';
import { h } from '../../dom';
import { tooltip } from '../../widgets/tooltip';

/**
 * The inspector's fields (docs/sheet/design.md §11): a number with its unit,
 * label over field as the style windows do (DESIGN.md §7.14), its ƒ button
 * (the value bound to data, design §7) inside its right edge. With several
 * items chosen a value they do not share is empty with “—” in it: what is
 * typed goes to all. ↑/↓ step by one (Shift ten, Alt a tenth), Enter takes
 * the value, Esc puts it back and keeps the key from the sheet behind.
 */

export interface NumberFieldOptions {
  readonly label: string;
  /** The value; null when the chosen items differ (“—”). */
  readonly value: number | null;
  readonly unit?: string;
  readonly decimals: number;
  readonly min?: number;
  readonly max?: number;
  /** Why it cannot be changed now; null when it can. */
  readonly readOnly: string | null;
  /** The expression it is bound to (ƒ pressed), or null. */
  readonly bound?: string | null;
  /** Its key (a property's path): the inspector gives the focus back to it after it is drawn again. */
  readonly key: string;
  onCommit(value: number): void;
  /** ƒ pressed: bind it or edit its binding. Without it there is no ƒ. */
  onBind?(): void;
  readonly wide?: boolean;
}

/** A number typed in the field: a comma is a decimal point too; anything else is not a number. */
export function parseNumber(text: string): number | null {
  const t = text.trim().replace(',', '.');
  if (!/^[-+]?(\d+\.?\d*|\.\d+)$/.test(t)) return null;
  const v = Number(t);
  return Number.isFinite(v) ? v : null;
}

export function numberField(o: NumberFieldOptions, d: DisposableStore): HTMLElement {
  const shown = o.value === null ? '' : fixed(o.value, o.decimals);
  const input = h('input', {
    class: 'sheet-field__input num',
    value: shown,
    placeholder: o.value === null ? '—' : null,
    inputmode: 'decimal',
    spellcheck: 'false',
    'aria-label': o.label,
    readonly: o.readOnly !== null || !!o.bound,
    dataset: { key: o.key, escape: 'local' },
  });
  const clamp = (v: number) => Math.min(o.max ?? Infinity, Math.max(o.min ?? -Infinity, v));
  const commit = () => {
    if (input.readOnly || input.value === shown) return;
    const v = parseNumber(input.value);
    if (v === null) {
      input.value = shown;
      return;
    }
    o.onCommit(clamp(v));
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
    } else if ((e.key === 'ArrowUp' || e.key === 'ArrowDown') && !input.readOnly) {
      e.preventDefault();
      const step = (e.shiftKey ? 10 : e.altKey ? 0.1 : 1) * (e.key === 'ArrowUp' ? 1 : -1);
      const base = parseNumber(input.value) ?? o.value ?? 0;
      o.onCommit(clamp(Math.round((base + step) * 1000) / 1000));
    }
  });
  input.addEventListener('blur', commit);
  const box = h('div', { class: 'sheet-field__box', dataset: o.bound ? { bound: '' } : {} }, input, o.unit ? h('span', { class: 'sheet-field__unit' }, o.unit) : null);
  if (o.readOnly) d.add(tooltip(box, () => ({ title: o.label, note: o.readOnly ?? undefined })));
  else if (o.bound) d.add(tooltip(box, () => ({ title: `${o.label}: veriye bağlı`, description: o.bound ?? undefined })));
  if (o.onBind) box.append(fxButton({ label: o.label, bound: o.bound ?? null, readOnly: o.readOnly, onClick: o.onBind }, d));
  // A div, not a label: a label would pass a click on ƒ on to the input.
  return h('div', { class: `sheet-field${o.wide ? ' sheet-field--wide' : ''}` }, h('span', { class: 'sheet-field__label', 'aria-hidden': 'true' }, o.label), box);
}

/** ƒ: binds a value to data with an expression (design §7); pressed (accent tone) while it is bound. */
export function fxButton(o: { label: string; bound: string | null; readOnly: string | null; onClick(): void }, d: DisposableStore): HTMLButtonElement {
  const b = h(
    'button',
    { class: 'sheet-fx', type: 'button', 'aria-label': `${o.label}: veriye bağla`, 'aria-pressed': String(!!o.bound), disabled: o.readOnly !== null },
    'ƒ',
  );
  b.addEventListener('click', (e) => {
    e.preventDefault();
    o.onClick();
  });
  d.add(
    tooltip(b, () => ({
      title: o.bound ? 'Veriye bağlı' : 'Veriye bağla',
      description: o.bound ? `${o.label} şu ifadeden gelir: ${o.bound}` : `${o.label} bir ifadeden gelsin: atlas nesnesinin alanı, pafta ya da proje değişkeni (@pafta_adi, @olcek …).`,
      note: o.readOnly ?? undefined,
    })),
  );
  return b;
}
