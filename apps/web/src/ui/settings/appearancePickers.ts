import { ACCENTS, DRAWING_FONTS, UI_FONTS, type AccentId, type UiFontId } from '../../app/appearance';
import type { DrawingFont } from '../../model/projectSettings';
import { h } from '../dom';
import { icon } from '../icons';

/**
 * Uygulama ayarları → Görünüm: the accent colour as swatches and the
 * typeface as cards, each card written in its own face. Radio groups: ←/→
 * move the choice, as in the other settings.
 */

function radioGroup<T extends string>(el: HTMLElement, ids: readonly T[], value: T, onChange: (id: T) => void): void {
  el.addEventListener('keydown', (e) => {
    const step = e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 0;
    if (!step) return;
    e.preventDefault();
    const at = ids.indexOf(value);
    onChange(ids[(at + step + ids.length) % ids.length]);
    queueMicrotask(() => el.querySelector<HTMLElement>('[aria-checked="true"]')?.focus());
  });
}

export function accentPicker(opts: { value: AccentId; onChange: (id: AccentId) => void }): HTMLElement {
  const group = h('div', { class: 'accent-pick', role: 'radiogroup', 'aria-label': 'Vurgu rengi' });
  for (const a of ACCENTS) {
    const on = a.id === opts.value;
    const b = h(
      'button',
      { class: 'accent-pick__opt', type: 'button', role: 'radio', 'aria-checked': String(on), tabindex: on ? '0' : '-1', dataset: { accent: a.id } },
      // Half the swatch is the dark theme's tone, half the light one's.
      h('span', { class: 'accent-pick__swatch', style: `--sw-dark: ${a.swatch.dark}; --sw-light: ${a.swatch.light}`, 'aria-hidden': 'true' }, on ? icon('check', 12) : null),
      h('span', { class: 'accent-pick__label' }, a.label),
    );
    b.addEventListener('click', () => opts.onChange(a.id));
    group.append(b);
  }
  radioGroup(
    group,
    ACCENTS.map((a) => a.id),
    opts.value,
    opts.onChange,
  );
  return group;
}

export function fontPicker(opts: { value: UiFontId; onChange: (id: UiFontId) => void }): HTMLElement {
  return typefaceCards('Yazı tipi', UI_FONTS, 'Ağ Şı İ 123', opts.value, opts.onChange);
}

/** The drawing's typeface, each card showing drawing text in it: an island, a parcel point, a length. */
export function drawingFontPicker(opts: { value: DrawingFont; onChange: (id: DrawingFont) => void }): HTMLElement {
  return typefaceCards('Çizim yazı tipi', DRAWING_FONTS, '1244 ada · 12.50 m', opts.value, opts.onChange, 'font-pick--drawing');
}

function typefaceCards<T extends string>(
  label: string,
  fonts: readonly { readonly id: T; readonly label: string; readonly family: string; readonly note: string }[],
  sample: string,
  value: T,
  onChange: (id: T) => void,
  extra = '',
): HTMLElement {
  const opts = { value, onChange };
  const group = h('div', { class: `font-pick ${extra}`.trim(), role: 'radiogroup', 'aria-label': label });
  for (const f of fonts) {
    const on = f.id === opts.value;
    const b = h(
      'button',
      { class: 'font-pick__card', type: 'button', role: 'radio', 'aria-checked': String(on), tabindex: on ? '0' : '-1', dataset: { font: f.id } },
      h('span', { class: 'font-pick__sample', style: `font-family: ${f.family}` }, sample),
      h('span', { class: 'font-pick__name', style: `font-family: ${f.family}` }, f.label),
      h('span', { class: 'font-pick__note' }, f.note),
    );
    b.addEventListener('click', () => opts.onChange(f.id));
    group.append(b);
  }
  radioGroup(
    group,
    fonts.map((f) => f.id),
    opts.value,
    opts.onChange,
  );
  return group;
}
