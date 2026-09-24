import { DisposableStore, listen } from '../../core/disposable';
import { formatChord } from '../../core/keymap';
import { h, overlayRoot } from '../dom';
import { icon } from '../icons';
import { hideTooltip } from './tooltip';

export interface MenuItem {
  kind?: 'item' | 'separator' | 'header';
  label?: string;
  icon?: string;
  /** Colour chip (layer colour). */
  swatch?: string;
  shortcut?: string;
  checked?: boolean;
  /** Radio-style check (dot) instead of a tick. */
  radio?: boolean;
  disabled?: boolean;
  /** Right-aligned secondary text (counts, units). */
  hint?: string;
  /** A second line under the label saying what the item does (the row grows). */
  detail?: string;
  run?: () => void;
  items?: MenuItem[] | (() => MenuItem[]);
}

export interface PopupOptions {
  placement?: 'below' | 'right' | 'point';
  minWidth?: number;
  onClose?: () => void;
  /** Root-level ←/→ (menubar moves to the neighbouring menu). */
  onHorizontal?: (dir: -1 | 1) => void;
  /** Element that should not count as "outside" (e.g. the menubar button). */
  owner?: HTMLElement;
}

type Anchor = DOMRect | { x: number; y: number };

/** Dropdown, submenu and context menu — one implementation for all. */
export class PopupMenu {
  private static root: PopupMenu | null = null;
  readonly el: HTMLElement;
  private readonly items: MenuItem[];
  private rows: (HTMLElement | null)[] = [];
  private active = -1;
  private child: PopupMenu | null = null;
  private readonly parent: PopupMenu | null;
  private readonly opts: PopupOptions;
  private readonly d = new DisposableStore();
  private hoverTimer = 0;

  static open(items: MenuItem[], anchor: Anchor, opts: PopupOptions = {}): PopupMenu {
    PopupMenu.root?.close();
    const m = new PopupMenu(items, anchor, opts, null);
    PopupMenu.root = m;
    return m;
  }

  static get isOpen(): boolean {
    return !!PopupMenu.root;
  }

  static closeAll(): void {
    PopupMenu.root?.close();
  }

  private constructor(items: MenuItem[], anchor: Anchor, opts: PopupOptions, parent: PopupMenu | null) {
    hideTooltip();
    this.items = items;
    this.opts = opts;
    this.parent = parent;
    this.el = h('div', { class: 'menu', role: 'menu', tabindex: '-1' });
    if (opts.minWidth) this.el.style.minWidth = `${opts.minWidth}px`;
    this.render();
    overlayRoot().append(this.el);
    this.position(anchor);
    if (!parent) {
      this.d.add(listen<KeyboardEvent>(window, 'keydown', (e) => this.leaf().onKey(e), true));
      this.d.add(
        listen<PointerEvent>(
          window,
          'pointerdown',
          (e) => {
            const t = e.target as Node;
            if (this.contains(t) || opts.owner?.contains(t)) return;
            this.close();
          },
          true,
        ),
      );
      this.d.add(listen(window, 'blur', () => this.close()));
      this.d.add(listen(window, 'resize', () => this.close()));
    }
  }

  private leaf(): PopupMenu {
    let m: PopupMenu = this;
    while (m.child) m = m.child;
    return m;
  }

  private contains(t: Node): boolean {
    return this.el.contains(t) || !!this.child?.contains(t);
  }

  private render(): void {
    this.rows = this.items.map((item, i) => {
      if (item.kind === 'separator') {
        this.el.append(h('div', { class: 'menu__sep', role: 'separator' }));
        return null;
      }
      if (item.kind === 'header') {
        this.el.append(h('div', { class: 'menu__header' }, item.label));
        return null;
      }
      const hasSub = !!item.items;
      const row = h(
        'div',
        {
          class: 'menu__item',
          role: item.radio ? 'menuitemradio' : item.checked !== undefined ? 'menuitemcheckbox' : 'menuitem',
          'aria-checked': item.checked !== undefined ? String(item.checked) : null,
          'aria-disabled': item.disabled ? 'true' : null,
          'aria-haspopup': hasSub ? 'menu' : null,
          'data-detail': item.detail ? '' : null,
        },
        h(
          'span',
          { class: 'menu__check' },
          item.checked ? (item.radio ? h('span', { class: 'menu__dot' }) : icon('check', 14)) : null,
        ),
        h('span', { class: 'menu__icon' }, item.swatch ? h('span', { class: 'swatch', style: `--swatch:${item.swatch}` }) : item.icon ? icon(item.icon, item.detail ? 22 : 16) : null),
        item.detail
          ? h('span', { class: 'menu__label menu__label--2' }, h('span', { class: 'menu__title' }, item.label), h('span', { class: 'menu__detail' }, item.detail))
          : h('span', { class: 'menu__label' }, item.label),
        item.hint ? h('span', { class: 'menu__hint' }, item.hint) : null,
        item.shortcut ? h('span', { class: 'menu__kbd' }, formatChord(item.shortcut)) : null,
        hasSub ? h('span', { class: 'menu__sub' }, icon('chevronRight', 14)) : null,
      );
      row.addEventListener('pointerenter', () => {
        this.setActive(i);
        clearTimeout(this.hoverTimer);
        this.hoverTimer = window.setTimeout(() => (hasSub ? this.openSub(i) : this.closeSub()), hasSub ? 90 : 160);
      });
      row.addEventListener('click', (e) => {
        e.stopPropagation();
        this.activate(i);
      });
      this.el.append(row);
      return row;
    });
  }

  private position(anchor: Anchor): void {
    const r = this.el.getBoundingClientRect();
    let x: number;
    let y: number;
    if (anchor instanceof DOMRect) {
      if (this.opts.placement === 'right') {
        x = anchor.right - 2;
        y = anchor.top - 5;
        if (x + r.width > innerWidth - 4) x = anchor.left - r.width + 2;
      } else {
        x = anchor.left;
        y = anchor.bottom + 2;
        if (y + r.height > innerHeight - 4) y = anchor.top - r.height - 2;
      }
    } else {
      x = anchor.x;
      y = anchor.y;
      if (x + r.width > innerWidth - 4) x = anchor.x - r.width;
      if (y + r.height > innerHeight - 4) y = anchor.y - r.height;
    }
    x = Math.max(4, Math.min(x, innerWidth - r.width - 4));
    y = Math.max(4, Math.min(y, innerHeight - r.height - 4));
    this.el.style.transform = `translate(${Math.round(x)}px, ${Math.round(y)}px)`;
  }

  private setActive(i: number): void {
    this.rows[this.active]?.removeAttribute('data-active');
    this.active = i;
    this.rows[i]?.setAttribute('data-active', '');
  }

  private move(dir: 1 | -1): void {
    const n = this.items.length;
    for (let k = 1; k <= n; k++) {
      const i = (this.active + dir * k + n * 2) % n;
      if (this.rows[i] && !this.items[i].disabled) return this.setActive(i);
    }
  }

  private openSub(i: number): void {
    const item = this.items[i];
    if (!item.items || item.disabled) return;
    if (this.child && this.child.anchorIndex === i) return;
    this.closeSub();
    const list = typeof item.items === 'function' ? item.items() : item.items;
    this.child = new PopupMenu(list, this.rows[i]!.getBoundingClientRect(), { placement: 'right' }, this);
    this.child.anchorIndex = i;
  }

  private anchorIndex = -1;

  private closeSub(): void {
    this.child?.destroy();
    this.child = null;
  }

  private activate(i: number): void {
    const item = this.items[i];
    if (!item || item.disabled) return;
    if (item.items) {
      this.openSub(i);
      this.child?.move(1);
      return;
    }
    this.rootMenu().close();
    item.run?.();
  }

  private rootMenu(): PopupMenu {
    let m: PopupMenu = this;
    while (m.parent) m = m.parent;
    return m;
  }

  private onKey(e: KeyboardEvent): void {
    const stop = () => {
      e.preventDefault();
      e.stopPropagation();
    };
    switch (e.key) {
      case 'ArrowDown':
        stop();
        return this.move(1);
      case 'ArrowUp':
        stop();
        return this.move(-1);
      case 'ArrowRight':
        stop();
        if (this.items[this.active]?.items) {
          this.openSub(this.active);
          this.child?.move(1);
        } else this.rootMenu().opts.onHorizontal?.(1);
        return;
      case 'ArrowLeft':
        stop();
        if (this.parent) {
          this.parent.closeSub();
        } else this.opts.onHorizontal?.(-1);
        return;
      case 'Enter':
      case ' ':
        stop();
        return this.activate(this.active);
      case 'Escape':
        stop();
        if (this.parent) this.parent.closeSub();
        else this.close();
        return;
      case 'Tab':
        stop();
        return this.rootMenu().close();
    }
  }

  /** Highlight the first enabled row (keyboard-opened menus). */
  focusFirst(): void {
    this.active = -1;
    this.move(1);
  }

  private destroy(): void {
    clearTimeout(this.hoverTimer);
    this.closeSub();
    this.d.dispose();
    this.el.remove();
  }

  close(): void {
    const root = this.rootMenu();
    root.destroy();
    if (PopupMenu.root === root) PopupMenu.root = null;
    root.opts.onClose?.();
  }
}
