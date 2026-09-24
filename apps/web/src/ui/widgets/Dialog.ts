import { DisposableStore, listen } from '../../core/disposable';
import { h, overlayRoot, type Child } from '../dom';
import { icon } from '../icons';
import { PopupMenu } from './PopupMenu';

/**
 * Modal dialog: Tab goes round its controls, Esc closes, the focus returns
 * where it was. A new dialog replaces the open one, unless it is opened
 * with `stack` (a symbol picker over a style window, a question over an
 * editor): then it sits on top and only the top dialog takes keys; closing
 * it returns to the one below. Questions use `confirmDialog` (confirm.ts).
 */
export class Dialog {
  private static readonly stack: Dialog[] = [];
  readonly el: HTMLElement;
  readonly body: HTMLElement;
  private readonly d = new DisposableStore();
  private readonly returnFocus: Element | null;
  private readonly onClose?: () => void;
  private readonly beforeClose?: () => boolean;

  /**
   * `beforeClose` may refuse a close asked by the user (Esc, ×, backdrop)
   * by returning false, e.g. to ask about unsaved changes first.
   */
  constructor(opts: { title: string; width?: number; className?: string; content: Child[]; footer?: Child[]; onClose?: () => void; beforeClose?: () => boolean; stack?: boolean }) {
    if (!opts.stack) for (const d of [...Dialog.stack].reverse()) d.close();
    Dialog.stack.push(this);
    this.returnFocus = document.activeElement;
    const close = h('button', { class: 'ibtn', type: 'button', 'aria-label': 'Kapat' }, icon('close', 16));
    this.body = h('div', { class: 'dialog__body' }, opts.content);
    const card = h(
      'div',
      { class: `dialog ${opts.className ?? ''}`, role: 'dialog', 'aria-modal': 'true', 'aria-label': opts.title, tabindex: '-1', style: opts.width ? `width:min(${opts.width}px, 100% - 32px)` : null },
      h('header', { class: 'dialog__head' }, h('h2', { class: 'dialog__title' }, opts.title), close),
      this.body,
      opts.footer ? h('footer', { class: 'dialog__foot' }, opts.footer) : null,
    );
    this.el = h('div', { class: 'dialog-backdrop' }, card);
    this.onClose = opts.onClose;
    this.beforeClose = opts.beforeClose;
    overlayRoot().append(this.el);
    card.focus();
    this.d.add(listen(close, 'click', () => this.request()));
    // Keys pressed inside the dialog never reach app shortcuts behind it.
    this.d.add(listen<KeyboardEvent>(card, 'keydown', (e) => e.stopPropagation()));
    this.d.add(
      listen<PointerEvent>(this.el, 'pointerdown', (e) => {
        if (e.target === this.el) this.request();
      }),
    );
    this.d.add(
      listen<KeyboardEvent>(
        window,
        'keydown',
        (e) => {
          // Only the top dialog listens; the ones below wait.
          if (Dialog.stack.at(-1) !== this) return;
          // An open menu (a dropdown in the form) takes its own Esc first, and so
          // does a field that undoes its own edit with Esc (data-escape="local").
          if (e.key === 'Escape' && (PopupMenu.isOpen || (document.activeElement as HTMLElement | null)?.dataset?.escape === 'local')) return;
          if (e.key === 'Escape') {
            e.preventDefault();
            e.stopPropagation();
            this.request();
            return;
          }
          // Tab goes round the top window: what is behind the backdrop cannot take the focus.
          if (e.key === 'Tab' && !e.ctrlKey && !e.altKey && !e.metaKey) wrapTab(card, e);
          // Keep app shortcuts from firing behind the modal.
          if (!card.contains(e.target as Node)) e.stopPropagation();
        },
        true,
      ),
    );
  }

  /** A close the user asked for; `beforeClose` may keep the dialog open. */
  request(): void {
    if (this.beforeClose && !this.beforeClose()) return;
    this.close();
  }

  close(): void {
    this.onClose?.();
    this.d.dispose();
    this.el.remove();
    const at = Dialog.stack.indexOf(this);
    if (at >= 0) Dialog.stack.splice(at, 1);
    (this.returnFocus as HTMLElement | null)?.focus?.();
  }
}

/**
 * Tab from the last control goes to the first, Shift+Tab from the first
 * (or from the card) to the last, and a focus outside comes in. Anything
 * else is the browser's own order; an open menu keeps its keys.
 */
function wrapTab(card: HTMLElement, e: KeyboardEvent): void {
  if (PopupMenu.isOpen) return;
  const all = [...card.querySelectorAll<HTMLElement>('button, [href], input, select, textarea, [tabindex]')].filter(
    (el) => el.tabIndex >= 0 && !(el as HTMLButtonElement).disabled && el.getClientRects().length > 0,
  );
  if (!all.length) return;
  const active = document.activeElement as HTMLElement | null;
  const at = active ? all.indexOf(active) : -1;
  let next: HTMLElement | undefined;
  if (!active || !card.contains(active)) next = e.shiftKey ? all.at(-1) : all[0];
  else if (e.shiftKey && (active === card || at === 0)) next = all.at(-1);
  else if (!e.shiftKey && at === all.length - 1) next = all[0];
  if (!next) return;
  e.preventDefault();
  next.focus();
}
