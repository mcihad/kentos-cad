// Dialogs (DESIGN.md §7.9): a modal over a dimmed page, its title, its body and its buttons at the bottom right; Esc
// and the close button dismiss it, focus stays inside, and it gives focus back to where it came from.
import { type Child, h } from '../lib/dom';
import { icon } from './icons';
import { button } from './kit';

export type Dialog = { close: () => void; body: HTMLElement; root: HTMLElement };

export function openDialog(opts: { title: string; lead?: Child; body: Child; footer?: Child[]; wide?: boolean; onClose?: () => void }): Dialog {
  const before = document.activeElement as HTMLElement | null;
  const body = h('div', { class: 'dialog-body' }, opts.body);
  const closeBtn = h('button', { class: 'icon-btn dialog-x', type: 'button', 'aria-label': 'Kapat', title: 'Kapat (Esc)' }, icon('close', 18));
  const box = h(
    'div',
    { class: `dialog ${opts.wide ? 'dialog-wide' : ''}`, role: 'dialog', 'aria-modal': 'true', 'aria-label': opts.title },
    h('div', { class: 'dialog-head' }, h('div', null, h('h2', null, opts.title), opts.lead ? h('p', { class: 'dialog-lead' }, opts.lead) : null), closeBtn),
    body,
    opts.footer?.length ? h('div', { class: 'dialog-foot' }, ...opts.footer) : null,
  );
  const root = h('div', { class: 'backdrop' }, box);
  let closed = false;
  const close = () => {
    if (closed) return;
    closed = true;
    root.remove();
    document.removeEventListener('keydown', onKey, true);
    opts.onClose?.();
    before?.focus?.();
  };
  const onKey = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.stopPropagation();
      close();
    } else if (e.key === 'Tab') {
      const focusable = Array.from(box.querySelectorAll<HTMLElement>('a[href],button:not([disabled]),input,select,textarea')).filter((el) => el.offsetParent !== null);
      if (!focusable.length) return;
      const [first, last] = [focusable[0], focusable[focusable.length - 1]];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  };
  closeBtn.addEventListener('click', close);
  root.addEventListener('mousedown', (e) => {
    if (e.target === root) close();
  });
  document.addEventListener('keydown', onKey, true);
  document.body.append(root);
  queueMicrotask(() => box.querySelector<HTMLElement>('input,select,textarea,button.btn-primary')?.focus());
  return { close, body, root };
}

/** Asks before a step that cannot be undone; resolves with the answer. */
export function confirm(opts: { title: string; text: Child; confirm: string; danger?: boolean }): Promise<boolean> {
  return new Promise((resolve) => {
    let answered = false;
    const d = openDialog({
      title: opts.title,
      body: h('p', { class: 'dialog-text' }, opts.text),
      footer: [
        button('Vazgeç', { kind: 'ghost', onClick: () => d.close() }),
        button(opts.confirm, {
          kind: opts.danger ? 'danger' : 'primary',
          onClick: () => {
            answered = true;
            d.close();
          },
        }),
      ],
      onClose: () => resolve(answered),
    });
  });
}
