import type { AppContext } from '../../app/context';
import { watchAll } from '../../core/signal';
import type { DisposableStore } from '../../core/disposable';
import { h } from '../dom';
import { icon } from '../icons';
import { tooltip } from './tooltip';

/**
 * Icon button bound to a command: reflects enabled / pressed state and shows
 * title + shortcut in its tooltip. Used by toolbar, toolbox and panels.
 */
export function commandButton(
  ctx: AppContext,
  id: string,
  d: DisposableStore,
  opts: { size?: number; className?: string; placement?: 'right' | 'bottom' | 'top'; label?: boolean } = {},
): HTMLButtonElement {
  const cmd = ctx.commands.get(id);
  const btn = h(
    'button',
    { class: opts.className ?? 'tbtn', type: 'button', 'aria-label': cmd?.title ?? id, dataset: { command: id } },
    icon(cmd?.icon ?? 'more', opts.size ?? 18),
    opts.label ? h('span', { class: 'tbtn__label' }, cmd?.title) : null,
  );
  btn.addEventListener('click', () => ctx.commands.execute(id));
  const sync = () => {
    btn.disabled = !ctx.commands.isEnabled(id);
    const checked = cmd?.isChecked?.();
    if (checked !== undefined) btn.setAttribute('aria-pressed', String(checked));
  };
  sync();
  if (cmd?.watch) d.add(watchAll(cmd.watch, sync));
  d.add(
    tooltip(
      btn,
      () => ({ title: cmd?.title ?? id, shortcut: ctx.keymap.chordFor(id), description: cmd?.description }),
      opts.placement ?? 'bottom',
    ),
  );
  return btn;
}
