import { WORKSPACES, type WorkspaceSpec } from '../../app/workspaces';
import type { Workspace } from '../../model/projectSettings';
import { h } from '../dom';
import { icon } from '../icons';

/**
 * The work modes as cards (Yeni proje, Proje ayarları → Genel): the mode's
 * picture, name, what it is for and three points. Modes announced but not
 * built are shown last, dimmed, with “Yakında”, and cannot be chosen. A
 * radio group: ←/→ (and ↑/↓) move between the modes that can be chosen.
 */
export function workspacePicker(opts: { value: Workspace; onChange: (id: Workspace) => void; compact?: boolean }): HTMLElement {
  const group = h('div', { class: `wspick${opts.compact ? ' wspick--compact' : ''}`, role: 'radiogroup', 'aria-label': 'Çalışma modu' });
  const ready = WORKSPACES.filter((w) => w.status === 'ready');
  const soon = WORKSPACES.filter((w) => w.status === 'soon');
  const cards = new Map<Workspace, HTMLButtonElement>();
  const select = (id: Workspace, focus: boolean) => {
    for (const [wid, c] of cards) {
      const on = wid === id;
      c.setAttribute('aria-checked', String(on));
      c.tabIndex = on ? 0 : -1;
    }
    if (focus) cards.get(id)?.focus();
    opts.onChange(id);
  };
  const card = (w: WorkspaceSpec): HTMLButtonElement => {
    const soonMode = w.status === 'soon';
    const on = w.id === opts.value;
    const c = h(
      'button',
      {
        class: 'wspick__card',
        type: 'button',
        role: 'radio',
        'aria-checked': String(on),
        'aria-disabled': soonMode ? 'true' : null,
        tabindex: on ? '0' : '-1',
        dataset: { mode: w.id, status: w.status },
      },
      h('span', { class: 'wspick__art', 'aria-hidden': 'true' }, icon(w.icon, 20)),
      h(
        'span',
        { class: 'wspick__head' },
        h('span', { class: 'wspick__name' }, w.label),
        soonMode ? h('span', { class: 'wspick__badge' }, 'Yakında') : h('span', { class: 'wspick__check', 'aria-hidden': 'true' }, icon('check', 12)),
      ),
      h('span', { class: 'wspick__title' }, w.title),
      opts.compact ? null : h('span', { class: 'wspick__desc' }, w.description),
      opts.compact ? null : h('ul', { class: 'wspick__points' }, ...w.highlights.map((p) => h('li', null, p))),
    );
    if (soonMode) c.title = `${w.label}: yakında. ${w.description}`;
    else c.addEventListener('click', () => select(w.id, false));
    cards.set(w.id, c);
    return c;
  };
  group.append(h('div', { class: 'wspick__row' }, ...ready.map(card)), h('div', { class: 'wspick__soon' }, h('span', { class: 'wspick__soonlabel' }, 'Yakında'), h('div', { class: 'wspick__row' }, ...soon.map(card))));
  group.addEventListener('keydown', (e) => {
    const step = e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 0;
    if (!step) return;
    e.preventDefault();
    const at = ready.findIndex((w) => cards.get(w.id)?.getAttribute('aria-checked') === 'true');
    select(ready[(at + step + ready.length) % ready.length].id, true);
  });
  return group;
}
