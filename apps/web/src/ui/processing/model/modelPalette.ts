import type { ProcessingRegistry } from '../../../processing/registry';
import { INPUT_TYPES, type ModelInputType } from '../../../processing/modelEdit';
import type { ProcessingTool } from '../../../processing/types';
import { h, replaceChildren } from '../../dom';
import { icon } from '../../icons';
import { tooltip } from '../../widgets/tooltip';
import type { Disposable } from '../../../core/disposable';

/**
 * Left column of the model designer: model inputs by type, and the tools
 * by category with a search. A click adds next to the selected box (and
 * connects to it); dragging drops the tool where it is released.
 */

export interface PaletteEvents {
  addInput(type: ModelInputType, label: string): void;
  addTool(toolId: string, client: { x: number; y: number } | null): void;
}

export function modelPalette(registry: ProcessingRegistry, events: PaletteEvents, subs: Disposable[]): HTMLElement {
  const search = h('input', { class: 'field field--search', type: 'search', placeholder: 'Araç ara', 'aria-label': 'Araç ara', spellcheck: 'false' });
  const list = h('div', { class: 'mpalette__tools' });

  const toolRow = (t: ProcessingTool) => {
    const row = h('button', { class: 'mpalette__tool', type: 'button' }, h('span', { class: 'mpalette__icon' }, icon(t.icon ?? 'processing', 15)), h('span', { class: 'mpalette__label' }, t.label));
    subs.push(tooltip(row, () => ({ title: t.label, description: t.description, note: 'Tıklayın ya da tuvale sürükleyin' }), 'right'));
    // Pointer drag rather than HTML drag and drop: the ghost follows the pointer exactly.
    row.addEventListener('pointerdown', (e) => {
      if (e.button !== 0) return;
      const start = { x: e.clientX, y: e.clientY };
      let ghost: HTMLElement | null = null;
      const move = (ev: PointerEvent) => {
        if (!ghost && Math.hypot(ev.clientX - start.x, ev.clientY - start.y) < 5) return;
        if (!ghost) {
          ghost = h('div', { class: 'mpalette__ghost' }, icon(t.icon ?? 'processing', 15), t.label);
          document.body.append(ghost);
        }
        ghost.style.transform = `translate(${ev.clientX + 10}px, ${ev.clientY + 8}px)`;
      };
      const up = (ev: PointerEvent) => {
        window.removeEventListener('pointermove', move);
        window.removeEventListener('pointerup', up);
        if (ghost) {
          ghost.remove();
          events.addTool(t.id, { x: ev.clientX, y: ev.clientY });
        } else events.addTool(t.id, null);
      };
      window.addEventListener('pointermove', move);
      window.addEventListener('pointerup', up);
    });
    return row;
  };

  const renderTools = () => {
    const q = search.value.trim();
    const hits = q ? new Set(registry.search(q).map((t) => t.id)) : null;
    const tree = registry.tree(hits ? (t) => hits.has(t.id) : undefined);
    const groups = tree.flatMap(function flat(n): HTMLElement[] {
      return [h('div', { class: 'mpalette__group' }, n.category.label), ...n.tools.map(toolRow), ...n.children.flatMap(flat)];
    });
    replaceChildren(list, groups.length ? groups : h('div', { class: 'mpalette__empty' }, 'Aramayla eşleşen araç yok.'));
  };
  search.addEventListener('input', renderTools);
  renderTools();

  const inputs = INPUT_TYPES.map((t) => {
    const b = h('button', { class: 'mpalette__input', type: 'button', title: t.description }, icon(t.icon, 15), h('span', null, t.label));
    b.addEventListener('click', () => events.addInput(t.type, t.label));
    return b;
  });

  return h(
    'aside',
    { class: 'mpalette', 'aria-label': 'Model parçaları' },
    h('div', { class: 'mpalette__title' }, 'Girdi ekle'),
    h('div', { class: 'mpalette__inputs' }, inputs),
    h('div', { class: 'mpalette__title' }, 'Araçlar'),
    h('div', { class: 'mpalette__search' }, h('span', { class: 'field-icon' }, icon('search', 14)), search),
    list,
    h('p', { class: 'mpalette__tip' }, 'Bir aracı tıklayın ya da tuvale sürükleyin. Seçili kutu varsa yeni adım ona bağlanır; bağlantıyı değiştirmek için kutunun sağındaki noktadan sürükleyin.'),
  );
}
