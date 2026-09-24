import { h, type Child } from '../dom';
import { icon } from '../icons';

/**
 * Small stateless form controls. Each takes the current value and an
 * onChange callback; the caller re-renders when its model changes.
 */

export interface Option<T> {
  value: T;
  label: string;
  hint?: string;
  disabled?: boolean;
}

/** Segmented control (single choice, role=radiogroup). */
export function segmented<T extends string>(opts: { label: string; options: Option<T>[]; value: T; onChange: (v: T) => void }): HTMLElement {
  const group = h('div', { class: 'seg', role: 'radiogroup', 'aria-label': opts.label });
  const buttons = opts.options.map((o) => {
    const b = h(
      'button',
      {
        class: 'seg__opt',
        type: 'button',
        role: 'radio',
        'aria-checked': String(o.value === opts.value),
        tabindex: o.value === opts.value ? '0' : '-1',
        disabled: o.disabled,
        title: o.hint,
      },
      o.label,
    );
    b.addEventListener('click', () => opts.onChange(o.value));
    return b;
  });
  group.addEventListener('keydown', (e) => {
    if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return;
    e.preventDefault();
    const enabled = opts.options.filter((o) => !o.disabled);
    const i = enabled.findIndex((o) => o.value === opts.value);
    const next = enabled[(i + (e.key === 'ArrowRight' ? 1 : -1) + enabled.length) % enabled.length];
    opts.onChange(next.value);
    queueMicrotask(() => (group.querySelector('[aria-checked="true"]') as HTMLElement | null)?.focus());
  });
  group.append(...buttons);
  return group;
}

/** On/off switch. */
export function toggleSwitch(opts: { label: string; checked: boolean; onChange: (v: boolean) => void; disabled?: boolean }): HTMLButtonElement {
  const b = h(
    'button',
    { class: 'switch', type: 'button', role: 'switch', 'aria-checked': String(opts.checked), 'aria-label': opts.label, disabled: opts.disabled },
    h('span', { class: 'switch__thumb' }),
  );
  b.addEventListener('click', () => opts.onChange(!opts.checked));
  return b;
}

/** Integer stepper with − / + buttons and direct entry. */
export function stepper(opts: { label: string; value: number; min: number; max: number; unit?: string; onChange: (v: number) => void }): HTMLElement {
  const clamp = (v: number) => Math.min(opts.max, Math.max(opts.min, Math.round(v)));
  const input = h('input', { class: 'stepper__input num', value: String(opts.value), inputmode: 'numeric', 'aria-label': opts.label, spellcheck: 'false' });
  const dec = h('button', { class: 'stepper__btn', type: 'button', 'aria-label': `${opts.label} azalt`, disabled: opts.value <= opts.min }, '−');
  const inc = h('button', { class: 'stepper__btn', type: 'button', 'aria-label': `${opts.label} artır`, disabled: opts.value >= opts.max }, '+');
  dec.addEventListener('click', () => opts.onChange(clamp(opts.value - 1)));
  inc.addEventListener('click', () => opts.onChange(clamp(opts.value + 1)));
  const commit = () => {
    const v = parseInt(input.value, 10);
    if (Number.isFinite(v) && clamp(v) !== opts.value) opts.onChange(clamp(v));
    else input.value = String(opts.value);
  };
  input.addEventListener('blur', commit);
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') commit();
    if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.preventDefault();
      opts.onChange(clamp(opts.value + (e.key === 'ArrowUp' ? 1 : -1)));
    }
  });
  return h('div', { class: 'stepper' }, dec, input, inc, opts.unit ? h('span', { class: 'stepper__unit' }, opts.unit) : null);
}

/** Label + description on the left, control on the right. */
export function settingRow(label: string, description: string | null, control: Child, opts: { stacked?: boolean } = {}): HTMLElement {
  return h(
    'div',
    { class: `srow${opts.stacked ? ' srow--stacked' : ''}` },
    h('div', { class: 'srow__text' }, h('div', { class: 'srow__label' }, label), description ? h('div', { class: 'srow__desc' }, description) : null),
    h('div', { class: 'srow__control' }, control),
  );
}

/** Inline note box (info / warning). */
export function note(kind: 'info' | 'warn', ...content: Child[]): HTMLElement {
  return h('div', { class: `note note--${kind}` }, icon(kind === 'warn' ? 'warning' : 'info', 16), h('div', { class: 'note__body' }, content));
}

/** Single-line text input that reports every keystroke (no re-render needed). */
export function textField(opts: { label: string; value: string; placeholder?: string; onChange: (v: string) => void }): HTMLInputElement {
  const input = h('input', { class: 'field field--setting', value: opts.value, placeholder: opts.placeholder, 'aria-label': opts.label, spellcheck: 'false' });
  input.addEventListener('input', () => opts.onChange(input.value));
  return input;
}
