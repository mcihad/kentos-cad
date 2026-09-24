import type { AppContext } from '../../app/context';
import type { DockTab } from '../../app/state';
import { h } from '../dom';
import { icon } from '../icons';

const TABS: { id: DockTab; label: string; icon: string }[] = [
  { id: 'layers', label: 'Katmanlar', icon: 'layers' },
  { id: 'processing', label: 'İşlemler', icon: 'processing' },
];

/**
 * Tab strip for the upper dock slot. Each panel in the slot carries its
 * own copy with itself selected; only the visible panel's strip is seen.
 */
export function dockTabs(ctx: AppContext, current: DockTab): HTMLElement {
  return h(
    'div',
    { class: 'panel__tabs', role: 'tablist', 'aria-label': 'Sağ panel' },
    TABS.map((t) => {
      const b = h('button', { class: 'tab', type: 'button', role: 'tab', 'aria-selected': String(t.id === current) }, icon(t.icon, 15), h('span', null, t.label));
      b.addEventListener('click', () => ctx.ui.dockTab.set(t.id));
      return b;
    }),
  );
}
