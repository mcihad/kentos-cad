import '../../styles/attachments.css';
import { attachable, attachmentViews, writeAttachments, type AttachmentView } from '../../app/attachments';
import type { AppContext } from '../../app/context';
import { listen } from '../../core/disposable';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';

/** The window's title, which a trace names it by. */
export const VIEWER_TITLE = 'Fotoğraf';

/** A data address's bytes as a blob address the browser opens in a tab (a tab does not open `data:`). */
function blobAddress(src: string): string {
  if (!src.startsWith('data:')) return src;
  const [head, body] = src.split(',', 2);
  const mime = /^data:([^;]+)/.exec(head)?.[1] ?? 'application/octet-stream';
  const bytes = Uint8Array.from(atob(body ?? ''), (c) => c.charCodeAt(0));
  return URL.createObjectURL(new Blob([bytes], { type: mime }));
}

/**
 * The photo viewer (docs/adr/0225 §2; the desktop's `attachments::viewer`): an object's photos one at a time, large,
 * with their name, “2/5”, where they are and their note, which is changed in place (one step “Ekler”; not on a locked
 * layer). Önceki and Sonraki (← and →) go round; Yeni sekmede aç shows the photo in a tab of its own. A linked photo the
 * browser cannot read shows a grey frame that says so.
 */
export function openPhotoViewer(ctx: AppContext, entityId: number, start: number): void {
  const photos = (): AttachmentView[] => {
    const e = ctx.doc.get(entityId);
    return e ? attachmentViews(ctx, e).filter((v) => v.kind === 'photo') : [];
  };
  let list = photos();
  if (!list.length) return void ctx.log.info('Nesnenin fotoğrafı yok.');
  let at = Math.max(0, list.findIndex((v) => v.index === start));
  const picture = h('div', { class: 'photo-viewer__picture' });
  const caption = h('div', { class: 'photo-viewer__caption' });
  const note = h('textarea', { class: 'field photo-viewer__note', rows: '2', 'aria-label': 'Not', placeholder: 'Not yok' }) as HTMLTextAreaElement;
  const saveNote = h('button', { class: 'btn btn--small', type: 'button', disabled: true }, 'Notu kaydet');
  const prev = h('button', { class: 'btn', type: 'button', title: 'Önceki (←)' }, icon('chevronLeft', 16), h('span', null, 'Önceki'));
  const next = h('button', { class: 'btn', type: 'button', title: 'Sonraki (→)' }, h('span', null, 'Sonraki'), icon('chevronRight', 16));
  const outside = h('button', { class: 'btn', type: 'button' }, icon('externalOpen', 16), h('span', null, 'Yeni sekmede aç'));
  const close = h('button', { class: 'btn btn--primary', type: 'button' }, 'Kapat');

  function show(): void {
    list = photos();
    if (!list.length) return void dialog.close();
    at = Math.min(at, list.length - 1);
    const v = list[at];
    const e = ctx.doc.get(entityId);
    replaceChildren(
      picture,
      v.src
        ? h('img', { class: 'photo-viewer__img', src: v.src, alt: v.attachment.name, draggable: 'false' })
        : h('div', { class: 'photo-viewer__missing' }, icon('photo', 32), h('span', null, 'Bağlı fotoğraf tarayıcıda okunamaz')),
    );
    replaceChildren(caption, h('b', { title: v.attachment.name }, v.attachment.name), h('span', { class: 'photo-viewer__count num' }, `${at + 1}/${list.length}`), h('span', { class: 'photo-viewer__source', title: v.source }, v.source));
    note.value = v.attachment.note ?? '';
    const editable = !!e && attachable(ctx, e);
    note.readOnly = !editable;
    saveNote.hidden = !editable;
    saveNote.disabled = true;
    prev.disabled = next.disabled = list.length < 2;
    outside.disabled = !v.src;
  }

  const dialog = new Dialog({
    title: VIEWER_TITLE,
    width: 960,
    className: 'dialog--photo-viewer',
    content: [picture, caption, h('div', { class: 'photo-viewer__notes' }, note, saveNote)],
    footer: [prev, next, outside, h('div', { class: 'dialog__spacer' }), close],
  });
  const step = (d: number) => {
    at = (at + d + list.length) % list.length;
    show();
  };
  prev.addEventListener('click', () => step(-1));
  next.addEventListener('click', () => step(1));
  close.addEventListener('click', () => dialog.close());
  outside.addEventListener('click', () => {
    const src = list[at]?.src;
    if (src) window.open(blobAddress(src), '_blank', 'noopener');
  });
  note.addEventListener('input', () => (saveNote.disabled = note.value === (list[at]?.attachment.note ?? '')));
  saveNote.addEventListener('click', () => {
    const e = ctx.doc.get(entityId);
    const v = list[at];
    if (!e?.attachments?.[v.index]) return;
    const text = note.value.replace(/\r\n?/g, '\n');
    const changed = e.attachments.map((x, i) => {
      if (i !== v.index) return x;
      const { note: _n, ...rest } = x;
      return text.trim() ? { ...rest, note: text } : rest;
    });
    if (writeAttachments(ctx, e, changed)) show();
  });
  listen<KeyboardEvent>(dialog.el, 'keydown', (e) => {
    if (document.activeElement === note) return;
    if (e.key === 'ArrowLeft') step(-1);
    else if (e.key === 'ArrowRight') step(1);
  });
  show();
}
