import type { AppContext } from '../../app/context';
import { listen, type DisposableStore } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import { h } from '../dom';
import { icon } from '../icons';
import { tooltip } from '../widgets/tooltip';

/**
 * The Tam ekran button at the right of the menu bar and of the ribbon's tab
 * row: four corners out, or in while the app fills the screen.
 */
export function fullscreenButton(ctx: AppContext, d: DisposableStore, className: string): HTMLButtonElement {
  const b = h('button', { class: className, type: 'button', dataset: { command: 'view.fullscreen' } });
  const cmd = ctx.commands.get('view.fullscreen');
  const sync = () => {
    const on = !!cmd?.isChecked?.();
    b.replaceChildren(icon(on ? 'fullscreenExit' : 'fullscreen', 16));
    b.setAttribute('aria-label', on ? 'Tam ekrandan çık' : 'Tam ekran');
    b.setAttribute('aria-pressed', String(on));
    b.disabled = !ctx.commands.isEnabled('view.fullscreen');
  };
  sync();
  if (cmd?.watch) d.add(watchAll(cmd.watch, sync));
  d.add(listen(b, 'click', () => ctx.commands.execute('view.fullscreen')));
  d.add(tooltip(b, () => ({ title: cmd?.isChecked?.() ? 'Tam ekrandan çık' : 'Tam ekran', shortcut: cmd?.isChecked?.() ? 'Esc' : undefined, description: 'Uygulamayı ekranın tamamına yayar; tarayıcının F11 tuşu da aynı işi yapar.' })));
  return b;
}
