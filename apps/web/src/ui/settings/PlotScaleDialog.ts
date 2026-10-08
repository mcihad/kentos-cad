/**
 * Ölçek yaz… (docs/adr/0205 §4): the project's plot scale typed as 1:N, N a whole number from 1 to 1 000 000 (dots
 * between its digits read too). Tamam sets it, and the texts, leaders, dimensions and tables at the old general height
 * follow it in one step (`setPlotScale`); Vazgeç, Esc, × and the backdrop change nothing. The desktop's is
 * `plot_scale.rs`'s window.
 */
import type { AppContext } from '../../app/context';
import { scaleText, setPlotScale } from '../../app/annotationScale';
import { h } from '../dom';
import { typedScale } from '../statusbar/scaleSelector';
import { Dialog } from '../widgets/Dialog';

/** The largest scale denominator that may be typed. */
export const MAX_PLOT_SCALE = 1_000_000;

/** A typed plot scale: a whole number from 1 to `MAX_PLOT_SCALE`; null for anything else. */
export function typedPlotScale(text: string): number | null {
  const n = typedScale(text);
  return n !== null && n <= MAX_PLOT_SCALE ? n : null;
}

export function openPlotScaleDialog(ctx: AppContext): void {
  const now = ctx.doc.settings.plotScale.value;
  const input = h('input', {
    class: 'field num plotscale__field',
    value: String(now),
    inputmode: 'numeric',
    spellcheck: 'false',
    'aria-label': 'Çizim ölçeği paydası',
  });
  const status = h('div', { class: 'plotscale__status', role: 'status' }, `Şimdi ${scaleText(now)}.`);
  const ok = h('button', { class: 'btn btn--primary', type: 'button' }, 'Tamam');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const check = (): number | null => {
    const n = typedPlotScale(input.value);
    input.toggleAttribute('data-invalid', n === null);
    status.textContent = n === null ? `1 ile ${MAX_PLOT_SCALE.toLocaleString('tr-TR')} arasında bir tam sayı yazın.` : n === now ? `Şimdi ${scaleText(now)}.` : `${scaleText(now)} → ${scaleText(n)}: genel boydaki yazılar yeni ölçeğe uyar.`;
    ok.disabled = n === null;
    return n;
  };
  const dialog = new Dialog({
    title: 'Çizim ölçeği',
    className: 'dialog--plotscale',
    width: 380,
    content: [h('label', { class: 'plotscale__row' }, h('span', { class: 'plotscale__prefix num' }, '1:'), input), status],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, ok],
  });
  const submit = () => {
    const n = check();
    if (n === null) return;
    dialog.close();
    setPlotScale(ctx, n);
  };
  input.addEventListener('input', check);
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') submit();
  });
  ok.addEventListener('click', submit);
  cancel.addEventListener('click', () => dialog.close());
  input.focus();
  input.select();
}
