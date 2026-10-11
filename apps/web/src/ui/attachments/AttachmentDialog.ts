import '../../styles/io.css';
import '../../styles/attachments.css';
import type { AppContext } from '../../app/context';
import type { Attachment } from '../../contracts/generated/Attachment';
import { attachmentProblem } from '../../model/attachmentRules';
import { h, replaceChildren } from '../dom';
import { field, summaryLine } from '../io/common';
import { Dialog } from '../widgets/Dialog';

/** The windows' titles, which a trace names them by. */
export const LINK_TITLE = 'Adres ekle';
export const EDIT_TITLE = 'Eki düzenle';

export type AttachmentMode = { mode: 'link' } | { mode: 'edit'; attachment: Attachment };

/**
 * Adres ekle and Eki düzenle (docs/adr/0225 §2; the desktop's `attachments::dialog`): an address's name, address and
 * note, or an attachment's name and note, checked as they are typed by the contract's rules (`attachmentProblem`).
 * Kaydet hands the attachment to `save`, which writes it (one step “Ekler”) and says whether it did.
 */
export function openAttachmentDialog(_ctx: AppContext, opts: AttachmentMode, save: (a: Attachment) => boolean): void {
  const editing = opts.mode === 'edit' ? opts.attachment : null;
  const name = h('input', { class: 'field', type: 'text', value: editing?.name ?? '', 'aria-label': 'Ad', placeholder: 'Kurum sayfası' }) as HTMLInputElement;
  const url = h('input', { class: 'field', type: 'text', value: '', 'aria-label': 'Adres', placeholder: 'https://', spellcheck: 'false' }) as HTMLInputElement;
  const note = h('textarea', { class: 'field attachment-dialog__note', rows: '3', 'aria-label': 'Not' }, editing?.note ?? '') as HTMLTextAreaElement;
  const summary = h('div', { class: 'io-summary' });
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const ok = h('button', { class: 'btn btn--primary', type: 'button' }, 'Kaydet');

  /** The attachment as typed: the note only when it says something; an address's own name or its host. */
  const made = (): Attachment => {
    const text = note.value.replace(/\r\n?/g, '\n');
    const keep = text.trim() ? { note: text } : {};
    if (editing) {
      const { note: _n, ...rest } = editing;
      return { ...rest, name: name.value.trim() || editing.name, ...keep };
    }
    const address = url.value.trim();
    let named = name.value.trim();
    if (!named) {
      try {
        named = new URL(address).host || address;
      } catch {
        named = address;
      }
    }
    return { name: named, url: address, ...keep };
  };
  const check = (): void => {
    const problem = attachmentProblem(made());
    ok.disabled = problem !== null;
    replaceChildren(summary, problem ? summaryLine('warn', `${problem.charAt(0).toUpperCase()}${problem.slice(1)}.`) : summaryLine('ok', editing ? 'Ekin adı ve notu yazılacak.' : 'Adres nesneye eklenecek.'));
  };
  const rows = editing
    ? [h('div', { class: 'io-row' }, field('Ad', name, undefined, 'grow')), h('div', { class: 'io-row' }, field('Not', note, 'Satırlar olabilir; en çok 1 000 karakter.', 'grow'))]
    : [
        h('div', { class: 'io-row' }, field('Adres', url, 'http:// ya da https:// ile başlar.', 'grow')),
        h('div', { class: 'io-row' }, field('Ad', name, 'Boşsa adresin sunucusu.', 'grow')),
        h('div', { class: 'io-row' }, field('Not', note, undefined, 'grow')),
      ];
  const dialog = new Dialog({ title: editing ? EDIT_TITLE : LINK_TITLE, width: 520, className: 'dialog--io dialog--attachment', content: [...rows, summary], footer: [h('div', { class: 'dialog__spacer' }), cancel, ok] });
  for (const input of [name, url, note]) input.addEventListener('input', check);
  for (const input of [name, url])
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' && !ok.disabled) {
        e.preventDefault();
        submit();
      }
    });
  const submit = () => {
    check();
    if (ok.disabled) return;
    if (save(made())) dialog.close();
  };
  ok.addEventListener('click', submit);
  cancel.addEventListener('click', () => dialog.close());
  check();
  (editing ? name : url).focus();
}
