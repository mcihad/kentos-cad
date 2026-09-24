import { Signal } from '../../core/signal';
import { Component } from '../Component';
import { h, type Child } from '../dom';
import { icon } from '../icons';

/** Titled, collapsible dock panel with header actions. */
export class Panel extends Component {
  readonly el: HTMLElement;
  readonly body: HTMLElement;
  readonly collapsed = new Signal(false);
  private readonly meta: HTMLElement;
  private readonly toggle: HTMLElement;

  constructor(opts: { title: string; icon?: string; actions?: Child[]; className?: string }) {
    super();
    const toggle = (this.toggle = h(
      'button',
      { class: 'panel__toggle', type: 'button', 'aria-expanded': 'true' },
      h('span', { class: 'panel__chev' }, icon('chevronDown', 14)),
      h('span', { class: 'panel__title' }, opts.title),
    ));
    this.meta = h('span', { class: 'panel__meta' });
    this.body = h('div', { class: 'panel__body' });
    this.el = h(
      'section',
      { class: `panel ${opts.className ?? ''}`, 'aria-label': opts.title },
      h('header', { class: 'panel__head' }, toggle, this.meta, h('div', { class: 'panel__actions' }, opts.actions ?? [])),
      this.body,
    );
    toggle.addEventListener('click', () => this.collapsed.set(!this.collapsed.value));
    this.d.add(
      this.collapsed.subscribe((c) => {
        this.el.toggleAttribute('data-collapsed', c);
        toggle.setAttribute('aria-expanded', String(!c));
      }),
    );
  }

  setMeta(text: string): void {
    this.meta.textContent = text;
  }

  /** Panels sharing a dock slot show a tab strip in place of their title (the chevron still folds). */
  setTabs(tabs: HTMLElement): void {
    this.toggle.after(tabs);
    this.el.querySelector('.panel__head')!.toggleAttribute('data-tabbed', true);
  }
}
