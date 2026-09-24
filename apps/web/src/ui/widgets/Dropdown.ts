import { h, replaceChildren, type Child } from '../dom';
import { icon } from '../icons';
import { PopupMenu, type MenuItem } from './PopupMenu';

/**
 * Compact select-like field for toolbars and property grids. The option list
 * is produced on open, so it always reflects the current model.
 */
export class Dropdown {
  readonly el: HTMLButtonElement;
  private readonly value: HTMLElement;

  constructor(opts: { label?: string; width?: number; className?: string; items: () => MenuItem[]; ariaLabel: string }) {
    this.value = h('span', { class: 'dropdown__value' });
    this.el = h(
      'button',
      { class: `dropdown ${opts.className ?? ''}`, type: 'button', 'aria-haspopup': 'listbox', 'aria-label': opts.ariaLabel },
      opts.label ? h('span', { class: 'dropdown__label' }, opts.label) : null,
      this.value,
      h('span', { class: 'dropdown__caret' }, icon('chevronDown', 14)),
    );
    if (opts.width) this.el.style.width = `calc(${opts.width}px * var(--ui-scale))`;
    const open = () => {
      this.el.setAttribute('aria-expanded', 'true');
      const r = this.el.getBoundingClientRect();
      const m = PopupMenu.open(opts.items(), r, {
        minWidth: r.width,
        owner: this.el,
        onClose: () => this.el.setAttribute('aria-expanded', 'false'),
      });
      return m;
    };
    this.el.addEventListener('pointerdown', (e) => {
      if (e.button !== 0) return;
      e.preventDefault();
      if (this.el.getAttribute('aria-expanded') === 'true') PopupMenu.closeAll();
      else open();
    });
    this.el.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ' || e.key === 'ArrowDown') {
        e.preventDefault();
        open().focusFirst();
      }
    });
  }

  set(...content: Child[]): void {
    replaceChildren(this.value, ...content);
  }
}
