import '../../styles/io.css';
import '../../styles/opening.css';
import { h } from '../dom';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';

/**
 * The open's progress window (TODOS.md FILE-20, docs/adr/0030): the file,
 * the project once it is known (name, objects, layers), the stage and a bar.
 * Modal from the start, so nothing is drawn or edited on the drawing about to
 * be replaced; shown only when the open takes a moment. Vazgeç, Esc, × and
 * the backdrop stop the open: the drawing on screen stays as it was.
 */

/** What an open shows while it runs. */
export interface OpeningView {
  /** The stage in words and how far the whole open is (0–1). */
  step(text: string, fraction: number): void;
  /** What the project is, once the file says it. */
  project(text: string): void;
  close(): void;
}

/** How long an open runs before its window shows (ms). */
const QUIET_MS = 250;

export function openingDialog(name: string, cancel: () => void): OpeningView {
  const project = h('p', { class: 'opening__project' });
  const bar = h('span');
  const stage = h('p', { class: 'opening__stage', role: 'status' }, 'Dosya okunuyor…');
  const stop = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  let closed = false;
  const dialog = new Dialog({
    title: 'Çizim açılıyor',
    width: 460,
    className: 'dialog--opening',
    stack: true,
    content: [
      h('div', { class: 'io-file' }, icon('fileOpen', 18), h('div', { class: 'io-file__text' }, h('span', { class: 'io-file__name', title: name }, name))),
      project,
      h('div', { class: 'opening__bar' }, bar),
      stage,
      h('p', { class: 'opening__note' }, 'Açık çizim, yenisi bütünüyle okunup denetlenene kadar olduğu gibi kalır; Vazgeç ona dokunmaz.'),
    ],
    footer: [h('div', { class: 'dialog__spacer' }), stop],
    onClose: () => {
      if (!closed) cancel();
    },
  });
  dialog.el.classList.add('opening--quiet');
  const show = setTimeout(() => dialog.el.classList.remove('opening--quiet'), QUIET_MS);
  stop.addEventListener('click', () => dialog.request());
  stop.focus();
  return {
    step(text, fraction) {
      stage.textContent = text;
      bar.style.width = `${Math.round(Math.min(1, Math.max(0, fraction)) * 100)}%`;
    },
    project(text) {
      project.textContent = text;
    },
    close() {
      closed = true;
      clearTimeout(show);
      dialog.close();
    },
  };
}
