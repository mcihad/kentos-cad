import type { DataDefined } from '../../model/style';
import { resolveColor, type CanvasPalette } from '../../render/color';
import { h, type Child } from '../dom';
import { PopupMenu } from '../widgets/PopupMenu';

/**
 * Form controls of the symbol designer. Every control reports each change
 * at once (the preview follows while typing); the form is not rebuilt, so
 * focus stays put. Values that may come from an expression per object
 * carry an "ƒ" switch: on, the value is an expression with a fallback.
 */

export interface FieldEnv {
  palette: CanvasPalette;
}

/** A labelled row: label on top, control below (the designer's right column). */
export function row(label: string, control: Child, hint?: string): HTMLElement {
  return h('div', { class: 'sdf' }, h('label', { class: 'sdf__label' }, label), h('div', { class: 'sdf__control' }, control), hint ? h('div', { class: 'sdf__hint' }, hint) : null);
}

/** Two controls side by side (X/Y offsets, width/height). */
export const pair = (a: Child, b: Child) => h('div', { class: 'sdf__pair' }, a, b);

const parseNum = (s: string) => {
  const v = Number(s.replace(',', '.'));
  return Number.isFinite(v) ? v : null;
};

export function numberInput(value: number, onChange: (v: number) => void, opts: { unit?: string; step?: number; min?: number; max?: number; label: string }): HTMLElement {
  const input = h('input', { class: 'field sdf__num num', value: fmt(value), inputmode: 'decimal', 'aria-label': opts.label, spellcheck: 'false' });
  const clamp = (v: number) => Math.min(opts.max ?? Infinity, Math.max(opts.min ?? -Infinity, v));
  input.addEventListener('input', () => {
    const v = parseNum(input.value);
    if (v !== null) onChange(clamp(v));
  });
  input.addEventListener('blur', () => {
    const v = parseNum(input.value);
    input.value = fmt(v === null ? value : clamp(v));
  });
  input.addEventListener('keydown', (e) => {
    if (e.key !== 'ArrowUp' && e.key !== 'ArrowDown') return;
    e.preventDefault();
    const step = (opts.step ?? 0.1) * (e.shiftKey ? 10 : 1);
    const v = clamp(Math.round(((parseNum(input.value) ?? 0) + (e.key === 'ArrowUp' ? step : -step)) * 1e6) / 1e6);
    input.value = fmt(v);
    onChange(v);
  });
  return h('div', { class: 'sdf__numwrap' }, input, opts.unit ? h('span', { class: 'sdf__unit' }, opts.unit) : null);
}

const fmt = (v: number) => String(Math.round(v * 1e6) / 1e6);

export function textInput(value: string, onChange: (v: string) => void, opts: { label: string; placeholder?: string; mono?: boolean }): HTMLInputElement {
  const input = h('input', { class: `field${opts.mono ? ' mono' : ''}`, value, 'aria-label': opts.label, placeholder: opts.placeholder ?? null, spellcheck: 'false' });
  input.addEventListener('input', () => onChange(input.value));
  return input;
}

export function select<T extends string>(value: T, options: { value: T; label: string }[], onChange: (v: T) => void, label: string): HTMLSelectElement {
  const el = h('select', { class: 'field sdf__select', 'aria-label': label }, options.map((o) => h('option', { value: o.value, selected: o.value === value }, o.label)));
  el.addEventListener('change', () => onChange(el.value as T));
  return el;
}

export function checkbox(value: boolean, onChange: (v: boolean) => void, label: string): HTMLElement {
  const input = h('input', { type: 'checkbox', checked: value, 'aria-label': label });
  input.addEventListener('change', () => onChange(input.checked));
  return h('label', { class: 'sdf__check' }, input, h('span', null, label));
}

/** On/off lengths as "4 1.5" (empty = continuous). */
export function dashInput(value: readonly number[] | null | undefined, onChange: (v: number[] | null) => void, label: string): HTMLElement {
  const input = h('input', { class: 'field mono', value: value?.join(' ') ?? '', placeholder: 'sürekli (ör. 4 1.5)', 'aria-label': label, spellcheck: 'false' });
  input.addEventListener('input', () => {
    const parts = input.value.split(/[\s;]+/).filter(Boolean).map(parseNum);
    if (!parts.length) return onChange(null);
    if (parts.every((p): p is number => p !== null && p >= 0) && parts.some((p) => p > 0)) onChange(parts.slice(0, 8));
  });
  return input;
}

const TOKENS: { value: string; label: string }[] = [
  { value: 'ink', label: 'Mürekkep (siyah / koyu temada beyaz)' },
  { value: 'paper', label: 'Kâğıt (beyaz / koyu temada zemin)' },
  { value: 'fg', label: 'Ana ön plan' },
  { value: 'fg-dim', label: 'İkincil ön plan' },
];

/**
 * A colour: swatch (the system picker), hex or theme token as text, and a
 * menu of tokens; `allowNone` adds "yok" (no fill / no outline).
 */
export function colorInput(value: string | null, onChange: (v: string | null) => void, env: FieldEnv, opts: { label: string; allowNone?: boolean }): HTMLElement {
  const text = h('input', { class: 'field mono sdf__hex', value: value ?? '', placeholder: opts.allowNone ? 'yok' : '#000000', 'aria-label': opts.label, spellcheck: 'false' });
  const swatch = h('button', { class: 'sdf__swatch', type: 'button', 'aria-label': `${opts.label}: renk seç` });
  const picker = h('input', { type: 'color', class: 'sdf__picker', tabindex: '-1', 'aria-hidden': 'true' });
  const paint = (v: string | null) => {
    swatch.style.background = v ? resolveColor(v, env.palette) : 'transparent';
    swatch.dataset.none = v ? '' : 'yes';
  };
  const valid = (v: string) => /^#[0-9a-f]{6}([0-9a-f]{2})?$/i.test(v) || TOKENS.some((t) => t.value === v);
  paint(value);
  text.addEventListener('input', () => {
    const v = text.value.trim();
    if (!v && opts.allowNone) {
      paint(null);
      return onChange(null);
    }
    if (valid(v)) {
      paint(v);
      onChange(v.startsWith('#') ? v.toUpperCase() : v);
    }
  });
  swatch.addEventListener('click', () => {
    const cur = text.value.trim();
    picker.value = /^#[0-9a-f]{6}/i.test(cur) ? cur.slice(0, 7) : '#000000';
    picker.click();
  });
  picker.addEventListener('input', () => {
    // Keep an alpha the colour already had.
    const alpha = /^#[0-9a-f]{8}$/i.test(text.value.trim()) ? text.value.trim().slice(7) : '';
    const v = (picker.value + alpha).toUpperCase();
    text.value = v;
    paint(v);
    onChange(v);
  });
  const more = h('button', { class: 'ibtn sdf__more', type: 'button', 'aria-label': `${opts.label}: tema renkleri` }, '⋯');
  more.addEventListener('click', () => {
    const r = more.getBoundingClientRect();
    PopupMenu.open(
      [
        ...TOKENS.map((t) => ({ label: t.label, swatch: resolveColor(t.value, env.palette), run: () => ((text.value = t.value), paint(t.value), onChange(t.value)) })),
        ...(opts.allowNone ? [{ kind: 'separator' as const }, { label: 'Yok', run: () => ((text.value = ''), paint(null), onChange(null)) }] : []),
      ],
      { x: r.left, y: r.bottom + 4 },
    );
  });
  return h('div', { class: 'sdf__color' }, swatch, picker, text, more);
}

// ── Data-defined values ────────────────────────────────────────────────

const isExpr = <T>(v: DataDefined<T> | undefined): v is { expr: string; fallback?: T } => !!v && typeof v === 'object' && 'expr' in v;

/**
 * A value that may come from an expression: the plain control, and an "ƒ"
 * switch that turns it into an expression field (the plain value becomes
 * the fallback for objects where the expression gives nothing).
 */
export function dataDefined<T>(value: DataDefined<T> | undefined, fallback: T, onChange: (v: DataDefined<T>) => void, plain: (v: T, set: (v: T) => void) => Child, label: string): HTMLElement {
  const host = h('div', { class: 'sdf__dd' });
  let current = value ?? fallback;
  const render = () => {
    const expr = isExpr(current);
    const fx = h('button', { class: 'sdf__fx', type: 'button', 'aria-pressed': String(expr), title: expr ? 'Sabit değere dön' : 'Nesnenin özniteliğinden ya da bir ifadeden al' }, 'ƒ');
    fx.addEventListener('click', () => {
      current = isExpr(current) ? (current.fallback ?? fallback) : { expr: '', fallback: current as T };
      onChange(current);
      render();
    });
    if (isExpr(current)) {
      const c = current;
      const input = textInput(c.expr, (v) => ((current = { expr: v, fallback: c.fallback }), onChange(current)), { label: `${label} ifadesi`, placeholder: '"Alan" ya da ifade', mono: true });
      host.replaceChildren(h('div', { class: 'sdf__ddrow' }, input, fx), h('div', { class: 'sdf__hint' }, 'İfade boş sonuç verirse sabit değer kullanılır.'));
      queueMicrotask(() => input.focus());
    } else host.replaceChildren(h('div', { class: 'sdf__ddrow' }, plain(current as T, (v) => ((current = v), onChange(v))), fx));
  };
  render();
  return host;
}
