import { h } from '../dom';
import { icon } from '../icons';
import { Dropdown } from './Dropdown';
import type { MenuItem } from './PopupMenu';

export type PropEditor =
  | { type: 'text' | 'number'; commit: (value: string) => void }
  | { type: 'select'; items: () => MenuItem[]; display: () => { text: string; swatch?: string } };

export interface PropRow {
  label: string;
  value: string;
  /** Numeric values use tabular figures and right alignment. */
  numeric?: boolean;
  unit?: string;
  editor?: PropEditor;
}

export interface PropSection {
  id: string;
  title: string;
  rows: PropRow[];
}

/** Two-column property grid; editable cells look like text until focused. */
export class PropertyGrid {
  readonly el = h('div', { class: 'props' });
  private collapsed = new Set<string>();

  render(sections: PropSection[]): void {
    const focusedLabel = (document.activeElement as HTMLElement | null)?.dataset?.propKey;
    this.el.textContent = '';
    for (const s of sections) {
      const isCollapsed = this.collapsed.has(s.id);
      const head = h(
        'button',
        { class: 'props__section', type: 'button', 'aria-expanded': String(!isCollapsed) },
        icon(isCollapsed ? 'chevronRight' : 'chevronDown', 14),
        h('span', null, s.title),
      );
      head.addEventListener('click', () => {
        isCollapsed ? this.collapsed.delete(s.id) : this.collapsed.add(s.id);
        this.render(sections);
      });
      this.el.append(head);
      if (isCollapsed) continue;
      const grid = h('dl', { class: 'props__grid' });
      for (const r of s.rows) {
        const key = `${s.id}:${r.label}`;
        grid.append(h('dt', { class: 'props__label', title: r.label }, r.label), h('dd', { class: 'props__value' }, this.cell(r, key)));
      }
      this.el.append(grid);
    }
    if (focusedLabel) this.el.querySelector<HTMLElement>(`[data-prop-key="${CSS.escape(focusedLabel)}"]`)?.focus();
  }

  private cell(r: PropRow, key: string): HTMLElement {
    const e = r.editor;
    if (!e) {
      return h('span', { class: `props__text${r.numeric ? ' num' : ''}`, title: r.value }, r.value, r.unit ? h('span', { class: 'props__unit' }, r.unit) : null);
    }
    if (e.type === 'select') {
      const dd = new Dropdown({ ariaLabel: r.label, className: 'dropdown--cell', items: e.items });
      const { text, swatch } = e.display();
      dd.set(swatch ? h('span', { class: 'swatch', style: `--swatch:${swatch}` }) : null, h('span', { class: 'dropdown__text' }, text));
      dd.el.dataset.propKey = key;
      return dd.el;
    }
    const input = h('input', {
      class: `props__input${r.numeric ? ' num' : ''}`,
      value: r.value,
      spellcheck: 'false',
      inputmode: e.type === 'number' ? 'decimal' : null,
      'aria-label': r.label,
      dataset: { propKey: key },
    });
    const commit = () => {
      if (input.value !== r.value) e.commit(input.value);
    };
    input.addEventListener('keydown', (ev) => {
      if (ev.key === 'Enter') {
        ev.preventDefault();
        commit();
        input.blur();
      } else if (ev.key === 'Escape') {
        ev.preventDefault();
        ev.stopPropagation();
        input.value = r.value;
        input.blur();
      }
    });
    input.addEventListener('blur', commit);
    return r.unit ? h('span', { class: 'props__with-unit' }, input, h('span', { class: 'props__unit' }, r.unit)) : input;
  }
}
