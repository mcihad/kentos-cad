// Forms (DESIGN.md §7.13, §10.1): a label above each control, its hint under it, and the server's refusal shown at the
// field it names (`ApiError.path`) or, when it names none, above the buttons. Nothing is checked only in the browser:
// the server's words are the ones shown.
import { ApiError } from '../api/client';
import { type Child, h } from '../lib/dom';

type Field = { label: string; name: string; hint?: Child; required?: boolean };

function wrap(f: Field, control: HTMLElement): HTMLElement {
  const id = `f-${f.name.replace(/[^a-z0-9]/gi, '-')}`;
  control.id = id;
  return h(
    'div',
    { class: 'field', dataset: { field: f.name } },
    h('label', { class: 'field-label', for: id }, f.label, f.required ? h('span', { class: 'field-req', 'aria-hidden': 'true' }, ' *') : null),
    control,
    f.hint ? h('div', { class: 'field-hint' }, f.hint) : null,
    h('div', { class: 'field-error', role: 'alert' }),
  );
}

export function textField(f: Field & { value?: string | null; type?: string; placeholder?: string; autocomplete?: string; maxlength?: number }): HTMLElement {
  const input = h('input', {
    class: 'input',
    name: f.name,
    type: f.type ?? 'text',
    value: f.value ?? '',
    placeholder: f.placeholder ?? null,
    autocomplete: f.autocomplete ?? 'off',
    maxlength: f.maxlength ?? null,
    required: f.required ?? false,
  });
  return wrap(f, input);
}

export type Option = { value: string; label: string };

export function selectField(f: Field & { value?: string | null; options: Option[]; placeholder?: string }): HTMLElement {
  const select = h(
    'select',
    { class: 'input', name: f.name, required: f.required ?? false },
    f.placeholder ? h('option', { value: '' }, f.placeholder) : null,
    ...f.options.map((o) => h('option', { value: o.value, selected: o.value === (f.value ?? '') }, o.label)),
  );
  return wrap(f, select);
}

export function textareaField(f: Field & { value?: string | null; rows?: number; placeholder?: string }): HTMLElement {
  const area = h('textarea', { class: 'input', name: f.name, rows: f.rows ?? 4, placeholder: f.placeholder ?? null, required: f.required ?? false });
  area.value = f.value ?? '';
  return wrap(f, area);
}

/** An on/off setting as a switch (DESIGN.md §3.3: an open switch is accent-filled). */
export function switchField(f: { label: string; name: string; checked: boolean; hint?: Child }): HTMLElement {
  const input = h('input', { type: 'checkbox', role: 'switch', name: f.name, class: 'switch-input', checked: f.checked });
  return h(
    'label',
    { class: 'switch-field', dataset: { field: f.name } },
    input,
    h('span', { class: 'switch', 'aria-hidden': 'true' }),
    h('span', { class: 'switch-text' }, h('span', { class: 'switch-label' }, f.label), f.hint ? h('span', { class: 'field-hint' }, f.hint) : null),
  );
}

export function checkbox(label: Child, name: string, checked: boolean, value = 'on'): HTMLElement {
  return h('label', { class: 'check' }, h('input', { type: 'checkbox', name, value, checked }), h('span', null, label));
}

/** The form's values by name; a checkbox group's checked values as a list under its name. */
export function values(form: HTMLFormElement): Record<string, string> {
  const out: Record<string, string> = {};
  for (const el of Array.from(form.elements)) {
    if (!(el instanceof HTMLInputElement || el instanceof HTMLSelectElement || el instanceof HTMLTextAreaElement) || !el.name) continue;
    if (el instanceof HTMLInputElement && el.type === 'checkbox') continue;
    if (el instanceof HTMLInputElement && el.type === 'radio' && !el.checked) continue;
    out[el.name] = el.value;
  }
  return out;
}

export function checked(form: HTMLFormElement, name: string): boolean {
  const el = form.elements.namedItem(name);
  return el instanceof HTMLInputElement && el.checked;
}

export function checkedValues(form: HTMLFormElement, name: string): string[] {
  return Array.from(form.querySelectorAll<HTMLInputElement>(`input[type=checkbox][name="${name}"]`))
    .filter((i) => i.checked)
    .map((i) => i.value);
}

/** Text or null when blank. */
export function orNull(v: string | undefined): string | null {
  const t = (v ?? '').trim();
  return t ? t : null;
}

/** Clears the form's earlier refusals. */
export function clearErrors(form: HTMLElement): void {
  form.querySelectorAll('.field.has-error').forEach((f) => f.classList.remove('has-error'));
  form.querySelectorAll('.field-error').forEach((e) => (e.textContent = ''));
  form.querySelector('.form-error')?.replaceChildren();
}

/** Shows a refusal at its field, or above the buttons. */
export function showError(form: HTMLElement, error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  const path = error instanceof ApiError ? error.path : null;
  const field = path ? (form.querySelector(`.field[data-field="${CSS.escape(path)}"]`) ?? form.querySelector(`[data-field="${CSS.escape(path)}"]`)) : null;
  if (field) {
    field.classList.add('has-error');
    const slot = field.querySelector('.field-error');
    if (slot) slot.textContent = message;
    field.querySelector<HTMLElement>('input,select,textarea')?.focus();
    return;
  }
  const slot = form.querySelector('.form-error');
  if (slot) slot.replaceChildren(h('div', { class: 'notice tone-danger', role: 'alert' }, h('div', { class: 'notice-text' }, message)));
}

/** A form that runs `submit` with its values, its buttons held while it runs, its refusal shown where it belongs. */
export function form(children: Child[], submit: (form: HTMLFormElement) => Promise<void>, opts: { class?: string } = {}): HTMLFormElement {
  const f = h('form', { class: `form ${opts.class ?? ''}`, novalidate: true }, ...children);
  f.addEventListener('submit', async (e) => {
    e.preventDefault();
    clearErrors(f);
    const buttons = Array.from(f.querySelectorAll<HTMLButtonElement>('button'));
    buttons.forEach((b) => (b.disabled = true));
    try {
      await submit(f);
    } catch (err) {
      showError(f, err);
    } finally {
      buttons.forEach((b) => (b.disabled = false));
    }
  });
  return f;
}

export function formError(): HTMLElement {
  return h('div', { class: 'form-error' });
}

export function grid(...children: Child[]): HTMLElement {
  return h('div', { class: 'form-grid' }, ...children);
}
