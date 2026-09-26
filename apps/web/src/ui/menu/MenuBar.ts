import type { AppContext } from '../../app/context';
import { MAIN_MENU, resolveMenu, visibleMenus, type TopMenu } from '../../app/menus';
import { filterOf } from '../../app/workspaces';
import { fullscreenButton } from '../shell/fullscreenButton';
import { brandButton } from '../shell/brandButton';
import { listen } from '../../core/disposable';
import { Component } from '../Component';
import { h } from '../dom';
import { icon } from '../icons';
import { fitBar, type BarFit } from '../widgets/fit';
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
  private fit: BarFit | null = null;

  constructor(ctx: AppContext, menus: TopMenu[] = MAIN_MENU) {
    super();
    this.ctx = ctx;
    this.menus = menus;
    const docName = h('span', { class: 'menubar__doc-name' });
    const dirty = h('span', { class: 'menubar__dirty', title: 'Kaydedilmemiş değişiklikler var' });
    const crs = h('button', { class: 'menubar__crs', type: 'button' }, icon('crs', 14), h('span', { class: 'menubar__crs-name' }));
    this.nav = h('nav', { class: 'menubar__menus' });
    this.el = h(
      'header',
      { class: 'menubar', role: 'menubar', 'aria-label': 'Ana menü' },
      brandButton(ctx, this.d, 'menubar__brand'),
      this.nav,
      h('div', { class: 'menubar__doc' }, dirty, docName),
      h('div', { class: 'menubar__right' }, crs, fullscreenButton(ctx, this.d, 'menubar__icon')),
    );

    // Every menu stays in view (DESIGN.md §7.1): at a narrow window and a large type scale the bar makes
    // room in steps, as the ribbon's tab row does: the “KentOS” word, then the CRS's name (its button and
    // tooltip stay), then the menus' padding. The document's name shrinks first by itself.
    const fit = (this.fit = fitBar(
      this.el,
      3,
      (level) => (this.el.dataset.tight = String(level)),
      () => this.nav.scrollWidth <= this.nav.clientWidth,
    ));
    this.d.add(fit.dispose);
    this.d.add(ctx.prefs.uiFont.subscribe(() => fit.refit()));
    this.d.add(listen(document.fonts, 'loadingdone', () => fit.refit()));

    // The work mode decides which menus show (app/workspaces.ts).
    this.d.add(ctx.doc.settings.workspace.subscribe(() => this.renderMenus(), true));
    this.d.add(ctx.doc.name.subscribe((n) => (docName.textContent = n), true));
    this.d.add(ctx.doc.dirty.subscribe((v) => dirty.toggleAttribute('hidden', !v), true));
    this.d.add(
      ctx.doc.crs.subscribe((c) => {
        crs.querySelector('.menubar__crs-name')!.textContent = c.name;
        this.fit?.refit();
      }, true),
    );
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
    this.fit?.refit();
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
