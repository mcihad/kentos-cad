import type { AppContext } from '../../app/context';
import { MAIN_MENU, resolveMenu, visibleMenus, type TopMenu } from '../../app/menus';
import { filterOf } from '../../app/workspaces';
import { listen } from '../../core/disposable';
import { Component } from '../Component';
import { h } from '../dom';
import { icon } from '../icons';
import { PopupMenu } from '../widgets/PopupMenu';
import { tooltip } from '../widgets/tooltip';

/**
 * Classic desktop menu bar: click opens, hovering switches while open,
 * ←/→ move between menus. Menus are resolved on open so state is current.
 */
export class MenuBar extends Component {
  readonly el: HTMLElement;
  private buttons: HTMLButtonElement[] = [];
  private openIndex = -1;
  private readonly ctx: AppContext;
  private readonly menus: TopMenu[];
  private readonly nav: HTMLElement;
  /** The menus the project's work mode shows. */
  private shown: TopMenu[] = [];

  constructor(ctx: AppContext, menus: TopMenu[] = MAIN_MENU) {
    super();
    this.ctx = ctx;
    this.menus = menus;
    const docName = h('span', { class: 'menubar__doc-name' });
    const dirty = h('span', { class: 'menubar__dirty', title: 'Kaydedilmemiş değişiklikler var' });
    const crs = h('button', { class: 'menubar__crs', type: 'button' }, icon('crs', 14), h('span'));
    this.nav = h('nav', { class: 'menubar__menus' });
    this.el = h(
      'header',
      { class: 'menubar', role: 'menubar', 'aria-label': 'Ana menü' },
      h('div', { class: 'menubar__brand', 'aria-hidden': 'true' }, brandMark(), h('span', { class: 'menubar__product' }, 'KentOS')),
      this.nav,
      h('div', { class: 'menubar__doc' }, dirty, docName),
      h('div', { class: 'menubar__right' }, crs),
    );

    // The work mode decides which menus show (app/workspaces.ts).
    this.d.add(ctx.doc.settings.workspace.subscribe(() => this.renderMenus(), true));
    this.d.add(ctx.doc.name.subscribe((n) => (docName.textContent = n), true));
    this.d.add(ctx.doc.dirty.subscribe((v) => dirty.toggleAttribute('hidden', !v), true));
    this.d.add(ctx.doc.crs.subscribe((c) => (crs.querySelector('span')!.textContent = c.name), true));
    this.d.add(listen(crs, 'click', () => ctx.commands.execute('crs.set')));
    this.d.add(tooltip(crs, () => ({ title: 'Koordinat sistemi', description: `EPSG:${ctx.doc.crs.value.srid}. Değiştirmek için tıklayın.` })));
  }

  private renderMenus(): void {
    PopupMenu.closeAll();
    this.shown = visibleMenus(filterOf(this.ctx), this.menus);
    this.buttons = this.shown.map((m, i) => {
      const b = h('button', { class: 'menubar__item', type: 'button', role: 'menuitem', 'aria-haspopup': 'menu', 'aria-expanded': 'false', dataset: { menu: m.id } }, m.label);
      b.addEventListener('pointerdown', (e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        this.openIndex === i ? PopupMenu.closeAll() : this.open(i, false);
      });
      b.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' || e.key === ' ' || e.key === 'ArrowDown') {
          e.preventDefault();
          this.open(i, true);
        }
      });
      b.addEventListener('pointerenter', () => {
        if (this.openIndex >= 0 && this.openIndex !== i) this.open(i, false);
      });
      return b;
    });
    this.nav.replaceChildren(...this.buttons);
  }

  private open(i: number, keyboard: boolean): void {
    const btn = this.buttons[i];
    this.buttons.forEach((b) => b.setAttribute('aria-expanded', 'false'));
    btn.setAttribute('aria-expanded', 'true');
    this.openIndex = i;
    const menu = PopupMenu.open(resolveMenu(this.ctx, this.shown[i].items), btn.getBoundingClientRect(), {
      minWidth: 240,
      owner: this.el,
      onClose: () => {
        if (this.openIndex === i) {
          this.openIndex = -1;
          btn.setAttribute('aria-expanded', 'false');
        }
      },
      onHorizontal: (dir) => {
        const next = (i + dir + this.buttons.length) % this.buttons.length;
        this.open(next, true);
        this.buttons[next].focus();
      },
    });
    if (keyboard) menu.focusFirst();
  }
}

/** KentOS mark: a K whose arms meet at a survey point. */
export function brandMark(): SVGSVGElement {
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('viewBox', '0 0 20 20');
  svg.setAttribute('width', '18');
  svg.setAttribute('height', '18');
  svg.setAttribute('aria-hidden', 'true');
  svg.classList.add('menubar__mark');
  svg.innerHTML =
    '<path d="M5.5 3.2v13.6" stroke="currentColor" stroke-width="2.1" stroke-linecap="round"/>' +
    '<path d="M15 3.4 7.4 10 15 16.6" fill="none" stroke="var(--c-accent)" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round"/>' +
    '<circle cx="7.4" cy="10" r="2.1" fill="var(--c-accent)"/>';
  return svg;
}
